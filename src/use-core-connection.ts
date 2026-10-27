import { useEffect, useState } from "react";
import { getCoreState, subscribeCoreState, type CoreConnection } from "./desktop-api";

export function useCoreConnection(): CoreConnection {
  const [state, setState] = useState<CoreConnection>({ state: "starting", code: null });
  useEffect(() => {
    let live = true;
    let eventReceived = false;
    const install = (value: CoreConnection) => { if (live) setState(value); };
    let unsubscribe = () => {};
    try {
      unsubscribe = subscribeCoreState((value) => { eventReceived = true; install(value); });
    } catch { install({ state: "disconnected", code: "DesktopBridgeUnavailable" }); }
    void getCoreState().then((value) => { if (!eventReceived) install(value); })
      .catch(() => install({ state: "disconnected", code: "DesktopBridgeUnavailable" }));
    return () => { live = false; unsubscribe(); };
  }, []);
  return state;
}
