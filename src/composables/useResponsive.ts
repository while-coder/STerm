// 视口断点：驱动桌面 / 移动端布局分支。模块级单例，全应用共享同一份响应式状态。
import { onBeforeUnmount, onMounted, ref } from "vue";

const MOBILE_QUERY = "(max-width: 860px)";

const isMobile = ref(false);
let mq: MediaQueryList | null = null;
let refCount = 0;

function sync() {
  isMobile.value = mq?.matches ?? false;
}

export function useResponsive() {
  onMounted(() => {
    if (refCount === 0) {
      mq = window.matchMedia(MOBILE_QUERY);
      mq.addEventListener("change", sync);
      sync();
    }
    refCount += 1;
  });

  onBeforeUnmount(() => {
    refCount -= 1;
    if (refCount === 0 && mq) {
      mq.removeEventListener("change", sync);
      mq = null;
    }
  });

  return { isMobile };
}
