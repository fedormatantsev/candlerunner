<script lang="ts">
	import type { IAccount } from 'src/models/Account';
	import type { IInstrument } from 'src/models/Instrument';

	export let instruments: IInstrument[];
	export let account: IAccount;
	export let closeAccount: CallableFunction;

	let positions: {
		positions: { figi: string; lots: number }[];
		currencies: { iso_currency: string; amount: number }[];
	} = {
		positions: [],
		currencies: []
	};

	let getInstrumentName = function (figi: string) {
		const instr = instruments.find((item) => item.figi === figi);
		if (instr) {
			return instr.display_name;
		}

		return 'UNKNOWN';
	};

	$: {
		async function fetchPositions() {
			const resp = await fetch('http://127.0.0.1:27001/list-positions', {
				method: 'POST',
				body: JSON.stringify({ account_id: account.id }),
				headers: { 'Content-type': 'application/json; charset=UTF-8' }
			});

			positions = await resp.json();
		}
		fetchPositions();
	}
</script>

<div
	class="px-4 w-full min-h-40 bg-gray-100 aspect-w-1 aspect-h-1 rounded-md overflow-hidden aspect-none"
>
	<div class="mt-4 flex">
		<h2 class="text-sm text-gray-600 font-normal">Account id:</h2>
		<h2 class="text-sm text-gray-600 font-medium pl-2">{account.id}</h2>
	</div>
	<div class="flex">
		<h2 class="text-sm text-gray-600 font-normal">Account name:</h2>
		<h2 class="text-sm text-gray-600 font-medium pl-2">{account.name}</h2>
	</div>
	<div class="flex">
		<h2 class="text-sm text-gray-600 font-normal">Environment:</h2>
		<h2 class="text-sm text-gray-600 font-medium pl-2">{account.environment}</h2>
	</div>
	<div class="flex mb-4">
		<h2 class="text-sm text-gray-600 font-normal">Access:</h2>
		<h2 class="text-sm text-gray-600 font-medium pl-2">{account.access_level}</h2>
	</div>
	<div class="flex mb-2">
		<h2 class="text-sm text-gray-600 font-normal">Positions</h2>
	</div>
	<div class="mb-4">
		{#each positions.positions as position (position.figi)}
			<div class="flex">
				<h2 class="text-sm text-gray-600 font-normal">Instrument:</h2>
				<h2 class="text-sm text-gray-600 font-medium pl-2">
					{getInstrumentName(position.figi)}
				</h2>
			</div>
			<div class="flex mb-2">
				<h2 class="text-sm text-gray-600 font-normal">Lots:</h2>
				<h2 class="text-sm text-gray-600 font-medium pl-2">{position.lots}</h2>
			</div>
		{/each}
	</div>
	<div class="flex mb-2">
		<h2 class="text-sm text-gray-600 font-normal">Currencies</h2>
	</div>
	<div class="mb-4">
		{#each positions.currencies as currency (currency.iso_currency)}
			<div class="flex">
				<h2 class="text-sm text-gray-600 font-normal">{currency.amount}</h2>
				<h2 class="text-sm text-gray-600 font-medium pl-2">{currency.iso_currency}</h2>
			</div>
		{/each}
	</div>
	{#if account.environment === 'Sandbox'}
		<div class="flex items-center gap-3 mb-4">
			<button
				class="bg-gray-300 text-gray-900 rounded-md inline-block px-3 py-2 hover:bg-gray-200 hover:text-gray-800 transition-colors"
				on:click={closeAccount(account.id)}
			>
				Pay In
			</button>
			<button
				class="bg-red-300 text-gray-900 rounded-md inline-block px-3 py-2 hover:bg-red-200 hover:text-gray-800 transition-colors"
				on:click={closeAccount(account.id)}
			>
				Close
			</button>
		</div>
	{/if}
</div>
