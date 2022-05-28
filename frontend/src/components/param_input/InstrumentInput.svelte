<script lang="ts">
	import { instrumentStore } from '../../stores/instrument_store';
	import type { ParamValue } from '../../models/Params';
	import type { IInstrument } from 'src/models/Instrument';

	export let paramValue: ParamValue | null = null;

	let inputValue: String = '';
	let filteredInstruments: IInstrument[] = [];
	let selectedInstrument: IInstrument | undefined = undefined;

	let setInstrument = function (instrument: IInstrument) {
		inputValue = instrument.display_name;
		paramValue = { Instrument: instrument.figi };
	};

	$: {
		if (inputValue.length > 0) {
			function filter(item: IInstrument) {
				return (
					item.display_name.toLowerCase().startsWith(inputValue.toLowerCase()) ||
					item.ticker.toLowerCase().startsWith(inputValue.toLowerCase())
				);
			}

			function match(item: IInstrument) {
				return item.display_name.toLowerCase() === inputValue.toLowerCase();
			}

			filteredInstruments = $instrumentStore.filter(filter);
			selectedInstrument = $instrumentStore.find(match);
		} else {
			filteredInstruments = [];
		}
	}
</script>

<input
	class="block w-full py-2 px-3 border border-gray-300 bg-white rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
	bind:value={inputValue}
/>
{#if !selectedInstrument && filteredInstruments.length > 0}
	<ul class="fixed bg-white mt-1 p-2 rounded-md border border-gray-300">
		{#each filteredInstruments as instrument (instrument.figi)}
			<li on:click={() => setInstrument(instrument)}>{instrument.display_name}</li>
		{/each}
	</ul>
{/if}
