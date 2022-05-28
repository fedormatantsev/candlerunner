<script lang="ts">
	import { IsFloatValue, IsInstrumentValue, type ParamValue } from '../models/Params';
	import {
		IsRealtimePositionManagerOptions,
		type IPositionManagerInstanceDefinition
	} from '../models/PositionManagerInstanceDefinition';

	import { instrumentStore } from '../stores/instrument_store';

	export let instanceDef: IPositionManagerInstanceDefinition;
	let params: { paramName: string; paramValue: any }[] = [];

	$: {
		function getParamValue(paramValue: ParamValue): any {
			if (IsInstrumentValue(paramValue)) {
				const instrument = $instrumentStore.find((item) => item.figi == paramValue.Instrument);

				return instrument ? instrument.display_name : 'UNKNOWN';
			}
			if (IsFloatValue(paramValue)) {
				return paramValue.Float;
			}

			return 'UNKNOWN PARAM TYPE';
		}

		params = Array.from(Object.entries(instanceDef.params), ([paramName, paramValue]) => ({
			paramName,
			paramValue: getParamValue(paramValue)
		}));
	}
</script>

<div class="px-4 w-full min-h-40 bg-gray-100 rounded-md overflow-hidden">
	<div class="mt-4 flex justify-between">
		<h1 class="text-lg font-bold">{instanceDef.position_manager_name}</h1>
	</div>
	<div class="mt-2">
		{#each params as param (param.paramName)}
			<div class="flex">
				<h2 class="text-sm text-gray-600 font-normal">{param.paramName}:</h2>
				<h2 class="text-sm text-gray-600 font-medium pl-2">{param.paramValue}</h2>
			</div>
		{/each}
		{#if IsRealtimePositionManagerOptions(instanceDef.options)}
			<div class="flex mt-2">
				<h2 class="text-sm text-gray-600 font-normal">Account id:</h2>
				<h2 class="text-sm text-gray-600 font-medium pl-2">
					{JSON.stringify(instanceDef.options.Realtime.account_id)}
				</h2>
			</div>
		{/if}
		<div class="flex mt-2 mb-4">
			<h2 class="text-sm text-gray-600 font-normal">Strategies:</h2>
			<h2 class="text-sm text-gray-600 font-medium pl-2">
				{JSON.stringify(instanceDef.strategies)}
			</h2>
		</div>
	</div>
</div>
