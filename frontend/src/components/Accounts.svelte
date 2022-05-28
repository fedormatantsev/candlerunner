<script lang="ts">
	import Plus from '../icons/Plus.svelte';
	import { accountsStore, fetchAccounts } from '../stores/accounts_store';
	import { instrumentStore } from '../stores/instrument_store';
	import AccountCard from './AccountCard.svelte';

	let openAccount = async function () {
		await fetch('http://127.0.0.1:27001/open-sandbox-account', {
			method: 'POST'
		});
		await fetchAccounts();
	};

	let closeAccount = async function (account_id: string) {
		await fetch('http://127.0.0.1:27001/close-sandbox-account', {
			method: 'POST',
			body: JSON.stringify({
				account_id: account_id
			}),
			headers: {
				'Content-type': 'application/json; charset=UTF-8'
			}
		});
		await fetchAccounts();
	};
</script>

<div class="mb-8">
	<div class="flex items-center gap-3">
		<h1 class="text-3xl font-medium">Accounts</h1>
		<button
			class="h-8 bg-green-400 text-gray-600 rounded-md aspect-square inline-block px-1 py-1 hover:bg-green-300 hover:text-gray-500 transition-colors"
			on:click={openAccount}
		>
			<Plus />
		</button>
	</div>

	<div class="mt-6 grid grid-cols-1 gap-y-10 gap-x-6 sm:grid-cols-2 lg:grid-cols-4 xl:gap-x-8">
		{#each $accountsStore as account (account.id)}
			<div class="group relative">
				<AccountCard {account} {closeAccount} instruments={$instrumentStore} />
			</div>
		{/each}
	</div>
</div>
