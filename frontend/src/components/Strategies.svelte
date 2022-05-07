<script lang="ts">
	import StrategyCard from './StrategyCard.svelte';
	import Plus from '../icons/Plus.svelte';
	import CreateInstanceDialog from './create_instance_dialog/CreateInstanceDialog.svelte';

	let strategyInstances: { instanceId: String; instanceDef: Object }[] = [];
	let showCreateDialog = false;

	let fetchInstances = async function () {
		const resp = await fetch('http://127.0.0.1:27001/list-strategy-instances');
		const json = await resp.json();
		strategyInstances = Array.from(Object.entries(json), ([instanceId, instanceDef]) => ({
			instanceId,
			instanceDef
		}));
	};

	fetchInstances();
</script>

<div>
	{#if showCreateDialog}
		<CreateInstanceDialog
			on:close={function () {
				fetchInstances();
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
		{#each strategyInstances as instance (instance.instanceId)}
			<div class="group relative">
				<StrategyCard strategyDef={instance.instanceDef} />
			</div>
		{/each}
	</div>
</div>
