<script lang="ts">
	import InstrumentInput from './InstrumentInput.svelte';

	export let paramType: String;
	export let paramValue: Object = {};
</script>

<div>
	{#if paramType === 'Instrument'}
		<InstrumentInput bind:paramValue={paramValue}/>
    {:else}
        <h1>UNKNOWN PARAM TYPE</h1>
	{/if}
</div>
