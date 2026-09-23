import { useEffect, useRef, type KeyboardEvent } from "react";

const focusable = "button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), a[href], [tabindex]:not([tabindex='-1'])";

export function useDialogFocus<T extends HTMLElement>(onClose: () => void, closeEnabled = true) {
  const ref = useRef<T>(null);
  const close = useRef(onClose);
  const enabled = useRef(closeEnabled);
  close.current = onClose;
  enabled.current = closeEnabled;

  useEffect(() => {
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const dialog = ref.current;
    const controls = dialog?.querySelectorAll<HTMLElement>(focusable);
    (Array.from(controls ?? []).find(element => element.getClientRects().length > 0) ?? dialog)?.focus();
    return () => {
      // A destructive operation can keep its trigger disabled until its async busy state clears.
      // Restore on the next paint, after React has committed the enabled trigger again.
      window.requestAnimationFrame(() => {
        if (previous?.isConnected && !document.querySelector('[aria-modal="true"]')) previous.focus();
      });
    };
  }, []);

  const onKeyDown = (event: KeyboardEvent<T>) => {
    if (event.key === "Escape") {
      event.stopPropagation();
      if (enabled.current) { event.preventDefault(); close.current(); }
    }
    if (event.key !== "Tab") return;
    const controls = Array.from(ref.current?.querySelectorAll<HTMLElement>(focusable) ?? [])
      .filter(element => element.getClientRects().length > 0);
    if (!controls.length) { event.preventDefault(); return; }
    const first = controls[0];
    const last = controls[controls.length - 1];
    if (event.shiftKey && (document.activeElement === first || !ref.current?.contains(document.activeElement))) {
      event.preventDefault(); last.focus();
    } else if (!event.shiftKey && (document.activeElement === last || !ref.current?.contains(document.activeElement))) {
      event.preventDefault(); first.focus();
    }
  };
  return { ref, onKeyDown };
}
