import type { ParamValue } from './Params'

export interface IRealtimePositionManagerOptions {
    account_id: string
}

export type PositionManagerInstanceOptions = { Realtime: IRealtimePositionManagerOptions } | { Backtest: {} };

export interface IPositionManagerInstanceDefinition {
    position_manager_name: string | null,
    strategies: string[],
    options: PositionManagerInstanceOptions,
    params: { [key: string]: ParamValue }
}

export function IsRealtimePositionManagerOptions(options: PositionManagerInstanceOptions): options is { Realtime: IRealtimePositionManagerOptions } {
    return options.hasOwnProperty('Realtime');
}
