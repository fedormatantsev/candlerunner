<script lang="ts">
	import FloatInput from './FloatInput.svelte';
	import InstrumentInput from './InstrumentInput.svelte';
	import type { ParamValue } from '../../models/Params';

	export let paramType: String;
	export let paramValue: ParamValue | null = null;
</script>

<div>
	{#if paramType === 'Instrument'}
		<InstrumentInput bind:paramValue />
	{:else if paramType === 'Float'}
		<FloatInput bind:paramValue />
	{:else}
		<h1>UNKNOWN PARAM TYPE</h1>
	{/if}
</div>
