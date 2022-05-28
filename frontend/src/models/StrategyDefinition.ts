import type { IParamDefinition } from "./Params";

export interface IStrategyDefinition {
    strategy_name: string,
    strategy_description: string,
    params: IParamDefinition[]
}
