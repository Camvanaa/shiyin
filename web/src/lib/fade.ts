/** 图片加载完成后加上 ok 类，配合 .fade 实现淡入。已缓存的图片立即显示。 */
export function fade(node: HTMLImageElement) {
  node.classList.add('fade');
  const done = () => node.classList.add('ok');
  const reset = () => {
    node.classList.remove('ok');
    if (node.complete && node.naturalWidth) done();
  };
  reset();
  node.addEventListener('load', done);
  node.addEventListener('error', done);
  // 虚拟列表复用节点时 src 会变化
  const mo = new MutationObserver(reset);
  mo.observe(node, { attributes: true, attributeFilter: ['src'] });
  return {
    destroy() {
      node.removeEventListener('load', done);
      node.removeEventListener('error', done);
      mo.disconnect();
    },
  };
}
