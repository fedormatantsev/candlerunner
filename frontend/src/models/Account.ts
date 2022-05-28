export interface IAccount {
    id: string,
    name: string,
    access_level: 'ReadOnly' | 'FullAccess',
    environment: 'Production' | 'Sandbox'
};
