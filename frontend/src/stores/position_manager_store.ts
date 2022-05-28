import type { IPositionManagerDefinition } from 'src/models/PositionManagerDefinition';
import { writable } from 'svelte/store';

export const positionManagerStore = writable<IPositionManagerDefinition[]>([]);

const fetchPositionManagers = async () => {
	const response = await fetch('http://127.0.0.1:27001/list-position-managers');
	const position_managers: IPositionManagerDefinition[] = await response.json();

	positionManagerStore.set(position_managers);
}

fetchPositionManagers();
