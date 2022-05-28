import type { IStrategyInstanceDefinition } from 'src/models/StrategyInstanceDefinition';
import { writable } from 'svelte/store';

interface IStrategyInstance {
    instanceId: string,
    instanceDef: IStrategyInstanceDefinition
}

export const strategyInstanceStore = writable<IStrategyInstance[]>([]);

export async function fetchStrategyInstances() {
    const resp = await fetch('http://127.0.0.1:27001/list-strategy-instances');
    const json: { [key: string]: IStrategyInstanceDefinition } = await resp.json();

    const instances = Array.from(Object.entries(json), ([instanceId, instanceDef]) => ({
        instanceId,
        instanceDef
    }));

    strategyInstanceStore.set(instances);
};

fetchStrategyInstances();
