<script lang="ts">
	import { IsInstrumentValue, type ParamValue } from '../../models/Params';
	import type { IStrategyInstanceDefinition } from '../../models/StrategyInstanceDefinition';

	import { instrumentStore } from '../../stores/instrument_store';

	export let strategyDef: IStrategyInstanceDefinition;
	export let selected: boolean = false;

	function paramVal(val: ParamValue): any {
		if (IsInstrumentValue(val)) {
			const instrument = $instrumentStore.find((item) => item.figi === val.Instrument);
			if (instrument) {
				return instrument.display_name;
			} else {
				return 'Unknown instrument';
			}
		}

		return 'UNKNOWN';
	}

	let paramsPreview = Object.entries(strategyDef.params).map(([k, v]) => ({
		k,
		v: paramVal(v)
	}));
</script>

<button
	on:click={function () {
		selected = !selected;
	}}
	class={'rounded-sm  px-3 py-2 flex items-center transition-colors ' +
		(selected ? 'bg-green-300 hover:bg-green-200' : 'bg-gray-100 hover:bg-gray-50')}
>
	<h2 class="text-sm font-medium text-gray-800">{strategyDef.strategy_name}</h2>
	{#each paramsPreview as param (param.k)}
		<h2 class="text-xs font-normal text-gray-600 ml-2">{param.k}</h2>
		<h2 class="text-xs font-medium text-gray-800 ml-1">{param.v}</h2>
	{/each}
</button>
