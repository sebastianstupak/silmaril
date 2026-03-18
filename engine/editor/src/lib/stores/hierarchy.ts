// Entity hierarchy store — updated via subscription events
// TODO: Implement with Svelte stores

export interface HierarchyNode {
  id: number;
  name: string;
  children: HierarchyNode[];
}
