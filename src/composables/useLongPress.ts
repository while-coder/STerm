// 长按手势（移动端等价右键）：仅对触摸 / 笔输入生效，鼠标走原生右键。
// 触发后 suppressed 短暂置真，供组件忽略随后的 click，避免长按又误触主操作。
import { ref } from "vue";

export function useLongPress(ms = 500) {
  const suppressed = ref(false);
  let timer: ReturnType<typeof setTimeout> | null = null;
  let sx = 0;
  let sy = 0;

  function clear() {
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
  }

  function start(e: PointerEvent, cb: (e: PointerEvent) => void) {
    if (e.pointerType === "mouse") return;
    sx = e.clientX;
    sy = e.clientY;
    clear();
    timer = setTimeout(() => {
      suppressed.value = true;
      cb(e);
    }, ms);
  }

  function move(e: PointerEvent) {
    if (timer && Math.hypot(e.clientX - sx, e.clientY - sy) > 10) clear();
  }

  function end() {
    clear();
    if (suppressed.value) setTimeout(() => (suppressed.value = false), 350);
  }

  return { suppressed, start, move, end };
}
