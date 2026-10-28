export interface Toast {
  id: number;
  kind: 'info' | 'success' | 'error';
  title: string;
  detail?: string;
  action?: { label: string; run: () => void };
}

let seq = 0;

class Toasts {
  items = $state<Toast[]>([]);

  push(t: Omit<Toast, 'id'>, ms = 3200) {
    const id = ++seq;
    // 最多同时显示 3 条
    this.items = [...this.items.slice(-2), { ...t, id }];
    setTimeout(() => this.dismiss(id), ms);
    return id;
  }

  dismiss(id: number) {
    this.items = this.items.filter((t) => t.id !== id);
  }

  info(title: string, detail?: string) {
    return this.push({ kind: 'info', title, detail });
  }
  success(title: string, detail?: string, action?: Toast['action']) {
    return this.push({ kind: 'success', title, detail, action }, action ? 5000 : 3200);
  }
  error(title: string, detail?: string) {
    return this.push({ kind: 'error', title, detail }, 5000);
  }
}

export const toast = new Toasts();
