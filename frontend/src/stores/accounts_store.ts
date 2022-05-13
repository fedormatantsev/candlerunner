import { writable } from 'svelte/store';

export const accountsStore = writable([]);

export const fetchAccounts = async () => {
    const response = await fetch('http://127.0.0.1:27001/list-accounts');
    const accounts = await response.json();

    accountsStore.set(accounts);
}

fetchAccounts();
