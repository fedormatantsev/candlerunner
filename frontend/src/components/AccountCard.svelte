<script lang="ts">
	export let account;
	export let closeAccount;
</script>

<div
	class="px-4 w-full min-h-40 bg-gray-200 aspect-w-1 aspect-h-1 rounded-md overflow-hidden aspect-none"
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
	{#if account.environment === 'Sandbox'}
		<div class="flex items-center gap-3 mb-4">
            <button
            class="bg-gray-400 text-gray-900 rounded-md inline-block px-3 py-2 hover:bg-gray-300 hover:text-gray-800 transition-colors"
            on:click={closeAccount(account.id)}
        >
            Pay In
        </button>
			<button
				class="bg-red-400 text-gray-900 rounded-md inline-block px-3 py-2 hover:bg-red-300 hover:text-gray-800 transition-colors"
				on:click={closeAccount(account.id)}
			>
				Close
			</button>
		</div>
	{/if}
</div>
