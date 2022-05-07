<script lang="ts">
	import { onDestroy } from 'svelte';

	import { instrumentStore } from '../../stores/instrument_store';

	export let paramValue: any = {};

	let inputValue: String = '';
	let instruments: { figi: String; ticker: String; display_name: String }[] = [];
	let filteredInstruments: { figi: String; ticker: String; display_name: String }[] = [];
	let selectedInstrument: { figi: String; ticker: String; display_name: String } | undefined =
		undefined;

	const unsubscribe = instrumentStore.subscribe((value) => (instruments = value));

	let setInstrument = function (instrument) {
		inputValue = instrument.display_name;
		paramValue = { Instrument: instrument.figi };
	};

	$: {
		if (inputValue.length > 0) {
			function filter(item) {
				return (
					item.display_name.toLowerCase().startsWith(inputValue.toLowerCase()) ||
					item.ticker.toLowerCase().startsWith(inputValue.toLowerCase())
				);
			}

			function match(item) {
				return item.display_name.toLowerCase() === inputValue.toLowerCase();
			}

			filteredInstruments = instruments.filter(filter);
			selectedInstrument = instruments.find(match);
		} else {
			filteredInstruments = [];
		}
	}

	onDestroy(unsubscribe);
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
