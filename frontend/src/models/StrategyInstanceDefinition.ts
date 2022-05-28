import type { ParamValue } from './Params'

export interface IStrategyInstanceDefinition {
    strategy_name: String | null,
    time_from: String | null,
    time_to: String | null,
    resolution: 'OneMinute' | 'OneHour' | 'OneDay',
    params: { [key: string]: ParamValue }
}
