import { onBeforeUnmount, onMounted, watch, type Ref } from 'vue';
import { resizeBrowser } from '@/api/workspace';
import { isTauriRuntime } from '@/services/tauri';

export function useNativeBrowser(
  host: Ref<HTMLElement | undefined>,
  visible: Ref<boolean>,
  onError: (error: unknown) => void,
) {
  let observer: ResizeObserver | undefined;
  let frame = 0;
  let queue = Promise.resolve();
  let last = '';
  const update = () => {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
      if (!isTauriRuntime()) return;
      const rect = host.value?.getBoundingClientRect();
      const bounds = {
        x: Math.max(0, rect?.x ?? 0),
        y: Math.max(0, rect?.y ?? 0),
        width: Math.max(0, rect?.width ?? 0),
        height: Math.max(0, rect?.height ?? 0),
      };
      const show = visible.value && !!rect;
      const key = JSON.stringify([bounds, show]);
      if (last === key) return;
      last = key;
      queue = queue
        .then(() => resizeBrowser(bounds, show))
        .catch((reason) => {
          last = '';
          onError(reason);
        });
    });
  };
  onMounted(() => {
    observer = new ResizeObserver(update);
    if (host.value) observer.observe(host.value);
    window.addEventListener('resize', update);
    update();
  });
  watch(
    host,
    (element, old) => {
      if (old) observer?.unobserve(old);
      if (element) observer?.observe(element);
      update();
    },
    { flush: 'post' },
  );
  watch(visible, update, { flush: 'post' });
  onBeforeUnmount(() => {
    observer?.disconnect();
    cancelAnimationFrame(frame);
    window.removeEventListener('resize', update);
  });
  return { update };
}
