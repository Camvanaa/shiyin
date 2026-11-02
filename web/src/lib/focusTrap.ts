/**
 * 模态焦点管理：容器出现时把焦点移入、Tab 循环限制在容器内、卸载后恢复到触发元素。
 * 用法：`<div use:focusTrap role="dialog" aria-modal="true">`（或 `use:focusTrap={{ initial }}`）。
 */
type Options = { initial?: 'first' | 'container' };

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function focusTrap(node: HTMLElement, options: Options = {}) {
  // 记录打开前的焦点，关闭后归还
  const previous = (document.activeElement instanceof HTMLElement ? document.activeElement : null);

  const list = () =>
    Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
      (el) => el.offsetParent !== null || el === document.activeElement,
    );

  function focusFirst() {
    const els = list();
    if (options.initial === 'container' || !els.length) {
      if (!node.hasAttribute('tabindex')) node.tabIndex = -1;
      node.focus({ preventScroll: true });
      return;
    }
    // 优先聚焦内容区的第一个可交互元素，跳过关闭按钮等头部操作
    const target = els.find((el) => !el.closest('header')) ?? els[0];
    target.focus({ preventScroll: true });
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key !== 'Tab') return;
    const els = list();
    if (!els.length) return;
    const first = els[0];
    const last = els[els.length - 1];
    const active = document.activeElement as HTMLElement | null;
    if (e.shiftKey && (active === first || !node.contains(active))) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && (active === last || !node.contains(active))) {
      e.preventDefault();
      first.focus();
    }
  }

  // 等一帧让进入动画/子节点就绪后再落焦点
  const raf = requestAnimationFrame(focusFirst);
  document.addEventListener('keydown', onKeydown, true);

  return {
    destroy() {
      cancelAnimationFrame(raf);
      document.removeEventListener('keydown', onKeydown, true);
      // 触发元素还在 DOM 里才恢复焦点，避免把焦点丢给已移除节点
      if (previous && previous.isConnected) previous.focus({ preventScroll: true });
    },
  };
}
