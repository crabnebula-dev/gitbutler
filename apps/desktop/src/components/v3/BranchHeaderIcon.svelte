<script lang="ts">
	import Icon from '@gitbutler/ui/Icon.svelte';
	import type iconsJson from '@gitbutler/ui/data/icons.json';

	interface Props {
		iconName: keyof typeof iconsJson;
		lineColor: string;
	}

	const { iconName, lineColor }: Props = $props();
</script>

<div class="stack__status gap">
	<div class="stack__status--icon" style:--bg-color={lineColor}>
		<Icon name={iconName} />
	</div>
</div>

<style>
	.stack__status {
		flex: 0 0 auto;
		align-self: stretch;
		padding-top: 11px;
		display: flex;
		flex-direction: column;
		gap: 3px;
		width: 42px;
		--clr-transparent: transparent;

		& .stack__status--icon {
			display: flex;
			align-items: center;
			justify-content: center;
			width: 22px;
			height: 26px;
			border-radius: var(--radius-m);
			background-color: var(--bg-color);
			color: #fff;
			margin: 0 auto;
		}
	}
</style>
