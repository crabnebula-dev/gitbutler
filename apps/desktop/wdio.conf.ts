import { TestRecorder } from './e2e/record.ts';
import { spawn, spawnSync, type ChildProcess } from 'node:child_process';
import os from 'node:os';
import path from 'node:path';
import type { Options, Frameworks } from '@wdio/types';

import { waitTauriDriverReady } from "@crabnebula/tauri-driver";
import { fileURLToPath } from "url";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
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
			// @ts-expect-error custom tauri capabilities
			maxInstances: 1,
			'tauri:options': {
				application: path.resolve(__dirname, '../../target/debug/gitbutler-tauri'),
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
		spawnSync(
			"pnpm",
			["tauri", "build", "--debug", "--features", "automation"].concat(
				process.platform === 'darwin' ? ["-b", "app"] : ["--no-bundle"]
			),
			{
				cwd: path.resolve(__dirname, '..'),
				stdio: 'inherit',
				shell: true
			});


			if (isMac) {
			if (!process.env.CN_API_KEY) {
				console.error('CN_API_KEY env var is not set, which is required for test-runner-backend.');
				process.exit(1);
			}
			testRunnerBackend = spawn('npm', ["run", "test-runner-backend"], {
				stdio: 'inherit',
				shell: true,
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
	beforeSession: () => {
		if (isMac) {
			// On macOS, we use the tauri-driver package directly.
			//
			console.info('Remote Webdriver URL is: ', process.env.REMOTE_WEBDRIVER_URL);
			tauriDriver = spawn('pnpm', ['tauri-driver'], {
				stdio: [null, process.stdout, process.stderr],
			});
			waitTauriDriverReady(tauriDriver);
			console.info('Tauri driver ready.');
		} else {
			(tauriDriver = spawn(path.resolve(os.homedir(), '.cargo', 'bin', 'tauri-driver'), [], {
				stdio: [null, process.stdout, process.stderr]
			}));
		}
	},

	afterSession: () => {
		tauriDriver.kill();
	}
};

async function sleep(ms: number) {
	return await new Promise((resolve) => setTimeout(resolve, ms));
}

