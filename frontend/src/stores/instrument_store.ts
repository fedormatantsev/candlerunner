import { writable } from 'svelte/store';
import type { IInstrument } from 'src/models/Instrument';

export const instrumentStore = writable<IInstrument[]>([]);

const fetchInstruments = async () => {
	const response = await fetch('http://127.0.0.1:27001/list-instruments');
	const instruments = await response.json();

	instrumentStore.set(instruments);
}

fetchInstruments();
