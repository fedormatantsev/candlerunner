<script lang="ts">
	import { createEventDispatcher, onDestroy } from 'svelte';
	import { strategyStore } from '../../stores/strategy_store';
	import ParamInput from '../param_input/ParamInput.svelte';
	import Dialog from '../Dialog.svelte';

	import type { IStrategyInstanceDefinition } from '../../models/StrategyInstanceDefinition';
	import type { IStrategyDefinition } from 'src/models/StrategyDefinition';

	let payload: IStrategyInstanceDefinition = {
		strategy_name: null,
		time_from: null,
		time_to: null,
		resolution: 'OneHour',
		params: {}
	};

	let strategyDef: IStrategyDefinition | undefined = undefined;

	const dispatch = createEventDispatcher();

	const close = function () {
		dispatch('close');
	};

	const instantiate = function () {
		if (payload.strategy_name != null && payload.time_from != null) {
			fetch('http://127.0.0.1:27001/instantiate-strategy', {
				method: 'POST',
				body: JSON.stringify(payload),
				headers: {
					'Content-type': 'application/json; charset=UTF-8'
				}
			}).then(function (_) {
				dispatch('close');
			});
		}
	};

	$: {
		if (payload.strategy_name != null) {
			strategyDef = $strategyStore.find((item) => item.strategy_name == payload.strategy_name);
			if (strategyDef) {
				// Filter params that are not a part of selected strategy params
				payload.params = Object.fromEntries(
					Object.entries(payload.params).filter(function ([paramName, _]) {
						return strategyDef?.params.find((item) => item.name == paramName);
					})
				);
			}
		}
	}
</script>

<Dialog>
	<div class="grid grid-cols-1">
		<div class="col-span-1 mb-4">
			<h1 class="font-bold text-xl">Instantiate Strategy</h1>
		</div>
		<div class="col-span-1 py-2">
			<h1 class="font-medium text-sm text-gray-700">Strategy</h1>
			<select
				bind:value={payload.strategy_name}
				class="mt-3 block w-full py-2 px-3 border border-gray-300 bg-white rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
			>
				{#each $strategyStore as def (def.strategy_name)}
					<option>{def.strategy_name}</option>
				{/each}
			</select>
		</div>
		<div class="col-span-1 py-2 mt-1">
			<h1 class="font-medium text-sm text-gray-700">Time from</h1>
			<input
				bind:value={payload.time_from}
				class="mt-3 block w-full py-2 px-3 border border-gray-300 bg-white rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
			/>
		</div>
		<div class="col-span-1 py-2 mt-1">
			<h1 class="font-medium text-sm text-gray-700">Time to</h1>
			<input
				bind:value={payload.time_to}
				class="mt-3 block w-full py-2 px-3 border border-gray-300 bg-white rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
			/>
		</div>
		<div class="col-span-1 py-2 mt-1">
			<h1 class="font-medium text-sm text-gray-700">Resolution</h1>
			<select
				bind:value={payload.resolution}
				class="mt-3 block w-full py-2 px-3 border border-gray-300 bg-white rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
			>
				<option>OneMinute</option>
				<option>OneHour</option>
				<option>OneDay</option>
			</select>
		</div>
		<div class="col-span-1 py-2 mt-1">
			<h1 class="font-medium text-sm text-gray-700">Params</h1>
			<div class="grid grid-cols-2 mt-3">
				{#if strategyDef}
					{#each strategyDef.params as param (param.name)}
						<div class="col-span-1">
							<h2 class="text-base font-medium">{param.name}</h2>
							<p class="text-xs text-gray-500">{param.description}</p>
						</div>
						<div class="col-span-1">
							<ParamInput
								paramType={param.param_type}
								bind:paramValue={payload.params[param.name]}
							/>
						</div>
					{/each}
				{/if}
			</div>
		</div>
		<div class="col-span-1 mt-14 flex gap-3 justify-end pl-24">
			<button
				on:click={close}
				class="bg-gray-300 rounded-md px-4 py-3 text-sm text-gray-800 hover:bg-gray-200 transition-colors"
				>Cancel</button
			>
			<button
				on:click={instantiate}
				class="bg-green-400 rounded-md px-4 py-3 text-sm text-gray-800 hover:bg-green-300 transition-colors"
				>Instantiate</button
			>
		</div>
	</div>
</Dialog>
