import type { IParamDefinition } from 'src/models/Params';
import type { IStrategyDefinition } from 'src/models/StrategyDefinition';
import { writable } from 'svelte/store';

export const strategyStore = writable<IStrategyDefinition[]>([]);

const fetchStrategies = async () => {
	const response = await fetch('http://127.0.0.1:27001/list-strategies');
	const strategies: {
		strategy_name: string,
		strategy_description: string,
		params: { [key: string]: IParamDefinition }
	}[] = await response.json();

	strategyStore.set(strategies.map(function (item) {
		const params: IParamDefinition[] = Object.entries(item.params).map(([_, paramDef]) => (paramDef));

		const def: IStrategyDefinition = {
			strategy_name: item.strategy_name,
			strategy_description: item.strategy_description,
			params
		};

		return def;
	}));
}

fetchStrategies();
