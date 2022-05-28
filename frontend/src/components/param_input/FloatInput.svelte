<script lang="ts">
	import type { ParamValue } from '../../models/Params';

	export let paramValue: ParamValue | null = null;

	let inputValue: number | null = null;

	$: {
		if (inputValue) {
			paramValue = { Float: inputValue };
		}
	}
</script>

<input
	class="block w-full py-2 px-3 border border-gray-300 bg-white rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
	bind:value={inputValue}
	type="number"
/>
