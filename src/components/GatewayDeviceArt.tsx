/** Fixed-view device silhouettes keep topology labels in the regular DOM. */
export function GatewayDeviceArt({ kind }: { kind: "client" | "hub" | "provider" }) {
  return (
    <svg className={`asb-gateway-device is-${kind}`} viewBox="0 0 120 100" fill="none" aria-hidden="true" focusable="false">
      <path className="asb-gateway-device-base" d="m10 77 49-25 51 25-49 23Z" />
      {kind === "client" ? <ClientDevice /> : kind === "hub" ? <HubDevice /> : <ProviderDevice />}
    </svg>
  );
}

function ClientDevice() {
  return (
    <>
      <path className="asb-gateway-device-side" d="m28 21 10-7h60v44l-10 7Z" />
      <rect className="asb-gateway-device-face" x="24" y="21" width="64" height="44" rx="4" />
      <rect className="asb-gateway-device-screen" x="30" y="27" width="52" height="30" rx="2" />
      <path className="asb-gateway-device-detail" d="m40 36 7 6-7 6m14 0h14M54 65v11m-15 4 15-4 21 4-15 5Z" />
      <path className="asb-gateway-device-edge" d="M38 14h60l-10 7" />
    </>
  );
}

function HubDevice() {
  return (
    <>
      <path className="asb-gateway-device-top" d="m16 35 44-24 45 24-44 25Z" />
      <path className="asb-gateway-device-face" d="M16 35v27l45 25V60Z" />
      <path className="asb-gateway-device-side" d="m61 60 44-25v27L61 87Z" />
      <path className="asb-gateway-device-detail" d="m43 34 17-9 18 9-17 10Zm17-9v-6m18 15 9 5m-26 5v7m-18-17-9 5" />
      <path className="asb-gateway-device-port" d="m27 49 7 4v7l-7-4Zm13 7 7 4v7l-7-4Zm32 9 7-4v7l-7 4Zm13-7 7-4v7l-7 4Z" />
      <path className="asb-gateway-device-edge" d="m16 35 45 25 44-25" />
    </>
  );
}

function ProviderDevice() {
  return (
    <>
      <path className="asb-gateway-device-top" d="m31 20 38-12 26 17-38 13Z" />
      <path className="asb-gateway-device-face" d="M31 20v49l26 18V38Z" />
      <path className="asb-gateway-device-side" d="m57 38 38-13v49L57 87Z" />
      <path className="asb-gateway-device-edge" d="m31 36 26 17 38-13M31 52l26 17 38-13" />
      <path className="asb-gateway-device-detail" d="m67 43 16-5m-16 21 16-5m-16 21 16-5" />
      <path className="asb-gateway-device-port" d="m38 34 5 3v4l-5-3Zm0 16 5 3v4l-5-3Zm0 16 5 3v4l-5-3Z" />
    </>
  );
}

export function GatewayTopologyFloor() {
  return (
    <svg className="asb-gateway-floor" viewBox="0 0 1000 400" preserveAspectRatio="none" aria-hidden="true" focusable="false">
      <path className="asb-gateway-floor-surface" d="M0 70 770 0l230 300-770 100Z" />
    </svg>
  );
}
