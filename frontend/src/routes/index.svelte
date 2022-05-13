<script lang="ts">
    import Strategies from "../components/Strategies.svelte";
	import Accounts from "../components/Accounts.svelte";
</script>

<div class="min-h-full">
	<nav class="bg-white">
		<div class="mx-auto px-4 sm:px-6 lg:px-8">
			<div class="flex items-center justify-between h-16">
				<div class="flex items-center">
					<div class="flex-shrink-0">
						<h1 class="font-bold text-xl">CANDLERUNNER</h1>
					</div>
				</div>
			</div>
		</div>
	</nav>
	<div class="container mx-auto py-6">
		<Accounts></Accounts>
        <Strategies></Strategies>
	</div>
</div>
