<script lang="ts">
	import { createEventDispatcher, onDestroy } from 'svelte';
	import { strategyStore } from '../../stores/strategy_store';
	import ParamInput from './ParamInput.svelte';

	let selectedStrategy: String | null = null;
	let selectedParams: { name: String; description: String }[] = [];
	let paramValues = {};
	let timeFrom: String = '';
	let timeTo: String = '';

	const dispatch = createEventDispatcher();

	const close = function () {
		dispatch('close');
	};

	const instantiate = function () {
		if (selectedStrategy != null) {
			let payload = {
				strategy_name: selectedStrategy,
				time_from: timeFrom,
				time_to: timeTo.length > 0 ? timeTo : null,
				resolution: 'OneHour',
				params: paramValues
			};

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
		if (selectedStrategy != null) {
			const strategy = $strategyStore.find((item) => item.strategy_name == selectedStrategy);
			if (strategy) {
				selectedParams = strategy.params;
				paramValues = Object.fromEntries(
					Object.entries(paramValues).filter(function ([paramName, _]) {
						return selectedParams.find((item) => item.name == paramName);
					})
				);
			}
		}
	}
</script>

<div class="fixed z-10 inset-0 bg-gray-800 bg-opacity-75">
	<div class="flex items-start h-full w-full justify-center">
		<div class="grid grid-cols-1 p-6 rounded-md mt-24 mx-auto bg-white">
			<div class="col-span-1 mb-4">
				<h1 class="font-bold text-xl">Instantiate Strategy</h1>
			</div>
			<div class="col-span-1 py-2">
				<h1 class="font-medium text-sm text-gray-700">Strategy</h1>
				<select
					bind:value={selectedStrategy}
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
					bind:value={timeFrom}
					class="mt-3 block w-full py-2 px-3 border border-gray-300 bg-white rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
				/>
			</div>
			<div class="col-span-1 py-2 mt-1">
				<h1 class="font-medium text-sm text-gray-700">Time to</h1>
				<input
					bind:value={timeTo}
					class="mt-3 block w-full py-2 px-3 border border-gray-300 bg-white rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
				/>
			</div>
			<div class="col-span-1 py-2 mt-1">
				<h1 class="font-medium text-sm text-gray-700">Params</h1>
				<div class="grid grid-cols-2 mt-3">
					{#each selectedParams as param (param.name)}
						<div class="col-span-1">
							<h2 class="text-base font-medium">{param.name}</h2>
							<p class="text-xs text-gray-500">{param.description}</p>
						</div>
						<div class="col-span-1">
							<ParamInput paramType={param.param_type} bind:paramValue={paramValues[param.name]} />
						</div>
					{/each}
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
	</div>
</div>
