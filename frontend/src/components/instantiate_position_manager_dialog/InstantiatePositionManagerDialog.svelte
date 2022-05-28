<script lang="ts">
	import { createEventDispatcher, onDestroy } from 'svelte';
	import Dialog from '../Dialog.svelte';
	import {
		type IPositionManagerInstanceDefinition,
		IsRealtimePositionManagerOptions
	} from '../../models/PositionManagerInstanceDefinition';
	import { positionManagerStore } from '../../stores/position_manager_store';
	import ParamInput from '../param_input/ParamInput.svelte';
	import { strategyInstanceStore } from '../../stores/strategy_instance_store';
	import StrategyPreview from './StrategyPreview.svelte';
	import { accountsStore } from '../../stores/accounts_store';
	import type { IPositionManagerDefinition } from 'src/models/PositionManagerDefinition';

	let payload: IPositionManagerInstanceDefinition = {
		position_manager_name: null,
		strategies: [],
		options: { Realtime: { account_id: '' } },
		params: {}
	};

	let positionManagerDef: IPositionManagerDefinition | undefined;

	const dispatch = createEventDispatcher();

	const close = function () {
		dispatch('close');
	};

	let selectedStrategies: { [key: string]: boolean } = {};

	$: {
		payload.strategies = Object.entries(selectedStrategies)
			.filter(([k, v]) => v === true)
			.map(([k, _]) => k);
	}

	const instantiate = function () {
		if (payload.position_manager_name != null && payload.strategies.length != 0) {
			fetch('http://127.0.0.1:27001/instantiate-position-manager', {
				method: 'POST',
				body: JSON.stringify(payload),
				headers: {
					'Content-type': 'application/json; charset=UTF-8'
				}
			}).then(function (_) {
				dispatch('close');
			});
		}
	};

	$: {
		if (payload.position_manager_name != null) {
			positionManagerDef = $positionManagerStore.find(
				(item) => item.position_manager_name === payload.position_manager_name
			);

			if (positionManagerDef) {
				// Filter params that are not a part of selected strategy params
				payload.params = Object.fromEntries(
					Object.entries(payload.params).filter(function ([paramName, _]) {
						return positionManagerDef?.params.find((item) => item.name == paramName);
					})
				);
			}
		}
	}
</script>

<Dialog>
	<div class="grid grid-cols-1">
		<div class="col-span-1 mb-4">
			<h1 class="font-bold text-xl">Instantiate Position Manager</h1>
		</div>
		<div class="col-span-1 py-2">
			<h1 class="font-medium text-sm text-gray-700">Position Manager</h1>
			<select
				bind:value={payload.position_manager_name}
				class="mt-3 block w-full py-2 px-3 border border-gray-300 bg-white rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
			>
				{#each $positionManagerStore as def (def.position_manager_name)}
					<option>{def.position_manager_name}</option>
				{/each}
			</select>
		</div>
		<div class="col-span-1 py-2 mt-1">
			<h1 class="font-medium text-sm text-gray-700">Strategies</h1>
			<div class="grid grid-cols-2 mt-3">
				<div class="col-span-2 mb-4 flex">
					{#each $strategyInstanceStore as strategy_instance (strategy_instance.instanceId)}
						<div class="p-1">
							<StrategyPreview
								strategyDef={strategy_instance.instanceDef}
								bind:selected={selectedStrategies[strategy_instance.instanceId]}
							/>
						</div>
					{/each}
				</div>
			</div>
		</div>
		{#if IsRealtimePositionManagerOptions(payload.options)}
			<div class="col-span-1 py-2 mt-1">
				<h1 class="font-medium text-sm text-gray-700">Account</h1>
				<select
					bind:value={payload.options.Realtime.account_id}
					class="mt-3 block w-full py-2 px-3 border border-gray-300 bg-white rounded-md shadow-sm focus:outline-none focus:ring-indigo-500 focus:border-indigo-500"
				>
					{#each $accountsStore as account (account.id)}
						<option>{account.id}</option>
					{/each}
				</select>
			</div>
		{/if}
		<div class="col-span-1 py-2 mt-1">
			<h1 class="font-medium text-sm text-gray-700">Params</h1>
			<div class="grid grid-cols-2 mt-3">
				{#if positionManagerDef}
					{#each positionManagerDef.params as param (param.name)}
						<div class="col-span-1 mb-4">
							<h2 class="text-base font-medium">{param.name}</h2>
							<p class="text-xs text-gray-500">{param.description}</p>
						</div>
						<div class="col-span-1 mb-4 pl-6">
							<ParamInput
								paramType={param.param_type}
								bind:paramValue={payload.params[param.name]}
							/>
						</div>
					{/each}
				{/if}
			</div>
		</div>
		<div class="col-span-1 mt-14 flex gap-3 justify-end pl-24">
			<button
				on:click={close}
				class="bg-gray-300 rounded-md px-4 py-3 text-sm text-gray-800 hover:bg-gray-200 transition-colors"
				>Cancel</button
			>
			<button
				on:click={instantiate}
				class="bg-green-400 rounded-md px-4 py-3 text-sm text-gray-800 hover:bg-green-300 transition-colors"
				>Instantiate</button
			>
		</div>
	</div>
</Dialog>
