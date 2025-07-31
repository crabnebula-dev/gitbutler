import { TestRecorder } from './e2e/record';
import { waitTauriDriverReady } from '@crabnebula/tauri-driver';
import { spawn, spawnSync, type ChildProcess } from 'node:child_process';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'url';
import type { Options, Frameworks } from '@wdio/types';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const isMac = process.platform === 'darwin';

console.info(`Running tests on ${isMac ? 'macOS' : 'non-macOS'} platform.`);
console.info(`Process platform: ${process.platform}`);

const videoRecorder = new TestRecorder();
let tauriDriver: ChildProcess;
let exit = false;
let testRunnerBackend: ChildProcess;

export const config: Options.WebdriverIO = {
	hostname: '127.0.0.1',
	port: 4444,
	specs: ['./e2e/tests/**/*.spec.ts'],
	maxInstances: 1,
	capabilities: [
		{
			maxInstances: 1,
			'tauri:options': {
				application: path.resolve(
					__dirname,
					isMac
						? '../../target/debug/bundle/macos/GitButler Dev.app'
						: '../../target/debug/gitbutler-tauri'
				)
			}
		}
	],
	reporters: ['spec'],
	framework: 'mocha',
	mochaOpts: {
		ui: 'bdd',
		timeout: 60000
	},
	autoCompileOpts: {
		autoCompile: true,
		tsNodeOpts: {
			project: './tsconfig.json',
			transpileOnly: true
		}
	},

	waitforTimeout: 10000,
	connectionRetryTimeout: 120000,
	connectionRetryCount: 0,

	onPrepare: function () {
		const config = {
			bundle: {
				createUpdaterArtifacts: false
			}
		};
		spawnSync(
			'pnpm',
			[
				'tauri',
				'build',
				'--debug',
				'--features',
				'automation',
				'--config',
				JSON.stringify(config)
			].concat(process.platform === 'darwin' ? ['-b', 'app'] : ['--no-bundle']),
			{
				cwd: path.resolve(__dirname, '..'),
				stdio: 'inherit',
				shell: process.platform === 'win32'
			}
		);

		if (isMac) {
			if (!process.env.CN_API_KEY) {
				console.error('CN_API_KEY env var is not set, which is required for test-runner-backend.');
				process.exit(1);
			}
			testRunnerBackend = spawn('pnpm', ['test-runner-backend'], {
				stdio: 'inherit',
				shell: true
			});
			testRunnerBackend.on('error', (error) => {
				console.error('Test runner backend error:', error);
				process.exit(1);
			});
			testRunnerBackend.on('exit', (code) => {
				if (!exit) {
					console.error('Test runner backend exited with code:', code);
					process.exit(1);
				}
			});
			process.env.REMOTE_WEBDRIVER_URL = 'http://127.0.0.1:3000';
		}
	},

	beforeTest: async function (test: Frameworks.Test) {
		const videoPath = path.join(import.meta.dirname, '/e2e/videos');
		videoRecorder.start(test, videoPath);
	},

	afterTest: async function () {
		await sleep(2000); // Let browser settle before stopping.
		videoRecorder.stop();
	},

	// ensure we are running `tauri-driver` before the session starts so that we can proxy the webdriver requests
	beforeSession: async () => {
		if (isMac) {
			// On macOS, we use the tauri-driver package directly.
			//
			console.info('Remote Webdriver URL is: ', process.env.REMOTE_WEBDRIVER_URL);
			tauriDriver = spawn('pnpm', ['tauri-driver'], {
				stdio: [null, process.stdout, process.stderr],
				shell: true
			});
			await waitTauriDriverReady(tauriDriver);
			console.info('Tauri driver ready.');
		} else {
			tauriDriver = spawn(path.resolve(os.homedir(), '.cargo', 'bin', 'tauri-driver'), [], {
				stdio: [null, process.stdout, process.stderr]
			});
		}
	},

	afterSession: () => {
		closeTauriDriver();
	},

	onComplete: () => {
		testRunnerBackend?.kill();
	}
};

function closeTauriDriver() {
	exit = true;
	tauriDriver?.kill();
}

async function sleep(ms: number) {
	return await new Promise((resolve) => setTimeout(resolve, ms));
}

export function onShutdown(fn: () => void) {
	const cleanup = () => {
		try {
			fn();
		} finally {
			process.exit();
		}
	};

	process.on('exit', cleanup);
	process.on('SIGINT', cleanup);
	process.on('SIGTERM', cleanup);
	process.on('SIGHUP', cleanup);
	process.on('SIGBREAK', cleanup);
}

onShutdown(closeTauriDriver);
