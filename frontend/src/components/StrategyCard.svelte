<script lang="ts">
import { onDestroy } from 'svelte';

	import { instrumentStore } from '../stores/instrument_store';

	export let strategyDef;
	let params = [];
    let instruments = [];

    const unsubscribe = instrumentStore.subscribe((value) => instruments=value);

	$: {
		function getParamValue(paramDef) {
			if (paramDef.hasOwnProperty('Instrument')) {
                const instrument = instruments.find((item) => item.figi == paramDef.Instrument);

                return instrument ? instrument.display_name : "UNKNOWN";
			}
		}

		params = Array.from(Object.entries(strategyDef.params), ([paramName, paramDef]) => ({
			paramName,
			paramValue: getParamValue(paramDef)
		}));
	}

    onDestroy(unsubscribe);
</script>

<div
	class="px-4 w-full min-h-40 bg-gray-200 aspect-w-1 aspect-h-1 rounded-md overflow-hidden h-40 aspect-none"
>
	<div class="mt-4 flex justify-between">
		<h1 class="text-lg font-bold">{strategyDef.strategy_name}</h1>
	</div>
	<div class="mt-2">
		{#each params as param (param.paramName)}
			<div class="flex">
				<h2 class="text-sm text-gray-600 font-normal">{param.paramName}:</h2>
				<h2 class="text-sm text-gray-600 font-medium pl-2">{param.paramValue}</h2>
			</div>
		{/each}
		<div class="flex">
			<h2 class="text-sm text-gray-600 font-normal">time from:</h2>
			<h2 class="text-sm text-gray-600 font-medium pl-2">{strategyDef.time_from}</h2>
		</div>
		{#if strategyDef.time_to}
			<div class="flex">
				<h2 class="text-sm text-gray-600 font-normal">time to:</h2>
				<h2 class="text-sm text-gray-600 font-medium pl-2">{strategyDef.time_to}</h2>
			</div>
		{/if}
	</div>
</div>
