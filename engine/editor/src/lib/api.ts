// Typed wrappers around Tauri invoke() and listen()
// TODO: Implement when Tauri JS API is available

export type EntityId = number;

export interface ComponentData {
  type_name: string;
  data: unknown;
}

export interface SubscriptionConfig {
  entity_id?: EntityId;
  throttle_ms?: number;
}

// Commands (Svelte -> Rust)
export async function createEntity(): Promise<EntityId> {
  // return await invoke('create_entity');
  throw new Error('Not implemented — Tauri not yet integrated');
}

export async function selectEntity(id: EntityId | null): Promise<void> {
  // return await invoke('select_entity', { id });
  throw new Error('Not implemented');
}

export async function setComponent(entityId: EntityId, name: string, data: unknown): Promise<void> {
  // return await invoke('set_component', { id: entityId, name, data });
  throw new Error('Not implemented');
}

// Subscriptions (Rust -> Svelte)
export async function subscribe(channel: string, config?: SubscriptionConfig): Promise<number> {
  // return await invoke('subscribe', { channel, config });
  throw new Error('Not implemented');
}

export async function unsubscribe(subscriptionId: number): Promise<void> {
  // return await invoke('unsubscribe', { subscription_id: subscriptionId });
  throw new Error('Not implemented');
}
