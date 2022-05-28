<script lang="ts">
	import { IsInstrumentValue, type ParamValue } from '../models/Params';
	import type { IStrategyInstanceDefinition } from '../models/StrategyInstanceDefinition';

	import { instrumentStore } from '../stores/instrument_store';

	export let instanceDef: IStrategyInstanceDefinition;
	let params: { paramName: string; paramValue: ParamValue }[] = [];

	$: {
		function getParamValue(paramValue: ParamValue): any {
			if (IsInstrumentValue(paramValue)) {
				const instrument = $instrumentStore.find((item) => item.figi == paramValue.Instrument);

				return instrument ? instrument.display_name : 'UNKNOWN';
			}

			return 'UNKNOWN PARAM TYPE';
		}

		params = Array.from(Object.entries(instanceDef.params), ([paramName, paramValue]) => ({
			paramName,
			paramValue: getParamValue(paramValue)
		}));
	}
</script>

<div
	class="px-4 w-full min-h-40 bg-gray-100 aspect-w-1 aspect-h-1 rounded-md overflow-hidden h-40 aspect-none"
>
	<div class="mt-4 flex justify-between">
		<h1 class="text-lg font-bold">{instanceDef.strategy_name}</h1>
	</div>
	<div class="mt-2">
		{#each params as param (param.paramName)}
			<div class="flex">
				<h2 class="text-sm text-gray-600 font-normal">{param.paramName}:</h2>
				<h2 class="text-sm text-gray-600 font-medium pl-2">{param.paramValue}</h2>
			</div>
		{/each}
		<div class="flex">
			<h2 class="text-sm text-gray-600 font-normal">Time from:</h2>
			<h2 class="text-sm text-gray-600 font-medium pl-2">{instanceDef.time_from}</h2>
		</div>
		{#if instanceDef.time_to}
			<div class="flex">
				<h2 class="text-sm text-gray-600 font-normal">Time to:</h2>
				<h2 class="text-sm text-gray-600 font-medium pl-2">{instanceDef.time_to}</h2>
			</div>
		{/if}
		<div class="flex">
			<h2 class="text-sm text-gray-600 font-normal">Resolution:</h2>
			<h2 class="text-sm text-gray-600 font-medium pl-2">{instanceDef.resolution}</h2>
		</div>
	</div>
</div>
