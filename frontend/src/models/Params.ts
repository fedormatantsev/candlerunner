export interface IInstrumentValue {
    Instrument: String
}

export interface IFloatValue {
    Float: number
}

export type ParamValue = IInstrumentValue | IFloatValue;

export function IsInstrumentValue(val: ParamValue): val is IInstrumentValue {
    return val.hasOwnProperty('Instrument');
}

export function IsFloatValue(val: ParamValue): val is IFloatValue {
    return val.hasOwnProperty('Float');
}

export interface IParamDefinition {
    name: string,
    description: string,
    param_type: 'Instrument' | 'Float',
    default_value: ParamValue | null
};
