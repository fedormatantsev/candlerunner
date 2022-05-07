import { writable } from 'svelte/store';

export const instrumentStore = writable([]);

const fetchInstruments = async () => {
	const response = await fetch('http://127.0.0.1:27001/list-instruments');
	const instruments = await response.json();

	instrumentStore.set(instruments);
}

fetchInstruments();
