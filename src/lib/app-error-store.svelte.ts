import { createAppErrorRegistry, type AppErrorStore } from '$lib/app-error';

export function createAppErrorStore(): AppErrorStore {
  const registry = createAppErrorRegistry();
  let current = $state(registry.current);

  return {
    get current() {
      return current;
    },
    show(message, identity) {
      registry.show(message, identity);
      current = registry.current;
    },
    dismiss(id: number) {
      registry.dismiss(id);
      current = registry.current;
    },
  };
}
