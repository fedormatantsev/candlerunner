<script lang="ts">
	import StrategyCard from './StrategyCard.svelte';
	import Plus from '../icons/Plus.svelte';
	import CreateStrategyInstanceDialog from './instantiate_strategy_dialog/InstantiateStrategyDialog.svelte';
	import { strategyInstanceStore, fetchStrategyInstances } from '../stores/strategy_instance_store';

	let showCreateDialog = false;
</script>

<div class="mb-8">
	{#if showCreateDialog}
		<CreateStrategyInstanceDialog
			on:close={function () {
				fetchStrategyInstances();
				showCreateDialog = false;
			}}
		/>
	{/if}

	<div class="flex items-center gap-3">
		<h1 class="text-3xl font-medium">Strategies</h1>
		<button
			class="h-8 bg-green-400 text-gray-600 rounded-md aspect-square inline-block px-1 py-1 hover:bg-green-300 hover:text-gray-500 transition-colors"
			on:click={function () {
				showCreateDialog = true;
			}}
		>
			<Plus />
		</button>
	</div>

	<div class="mt-6 grid grid-cols-1 gap-y-10 gap-x-6 sm:grid-cols-2 lg:grid-cols-4 xl:gap-x-8">
		{#each $strategyInstanceStore as instance (instance.instanceId)}
			<div class="group relative">
				<StrategyCard instanceDef={instance.instanceDef} />
			</div>
		{/each}
	</div>
</div>
