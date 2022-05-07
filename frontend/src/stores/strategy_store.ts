import { writable } from 'svelte/store';

export const strategyStore = writable([]);

const fetchStrategies = async () => {
	const response = await fetch('http://127.0.0.1:27001/list-strategies');
	const strategies = await response.json();

	strategyStore.set(strategies);
}

fetchStrategies();
