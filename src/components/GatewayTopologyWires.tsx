import { useId, useLayoutEffect, useState, type RefObject } from "react";
import type { AppKind, GatewayRouteStatus } from "../api/client";
import { NATIVE_PROTOCOL, isProtocolTranslation } from "../lib/protocol";
import type { ClientFilterValue } from "./ClientFilter";

interface Anchor {
  left: number;
  right: number;
  y: number;
}

interface Geometry {
  width: number;
  height: number;
  anchors: Record<string, Anchor>;
}

interface Props {
  sceneRef: RefObject<HTMLDivElement | null>;
  routes: GatewayRouteStatus[];
  focus: ClientFilterValue;
  failed: boolean;
}

/** Wire endpoints follow rendered device bounds, including wrapping and zoom. */
export function GatewayTopologyWires({ sceneRef, routes, focus, failed }: Props) {
  const markerId = `gateway-direction-${useId().replace(/:/g, "")}`;
  const geometry = useDeviceGeometry(sceneRef, routes);
  const hub = geometry.anchors.hub;
  if (!hub || geometry.width === 0 || geometry.height === 0) return null;
  const clients = [...new Set(routes.map((route) => route.app))];
  return (
    <svg className={`asb-gateway-wires${failed ? " is-failure" : ""}`}
      viewBox={`0 0 ${geometry.width} ${geometry.height}`} aria-hidden="true" focusable="false">
      <defs>
        <marker id={markerId} viewBox="0 0 8 8" refX="7" refY="4" markerWidth="6" markerHeight="6" orient="auto">
          <path d="m1 1 6 3-6 3" fill="none" stroke="context-stroke" strokeWidth="1.5" />
        </marker>
      </defs>
      {clients.map((app) => {
        const client = geometry.anchors[`client:${app}`];
        return client && <Wire key={app} app={app} focus={focus} markerId={markerId}
          from={{ x: client.right, y: client.y }} to={{ x: hub.left, y: hub.y }} />;
      })}
      {routes.map((route) => {
        const provider = geometry.anchors[`${route.app}:${route.profileId}`];
        return provider && <Wire key={`${route.app}:${route.profileId}`} app={route.app} focus={focus} markerId={markerId}
          translated={isProtocolTranslation(NATIVE_PROTOCOL[route.app], route.upstreamProtocol)}
          from={{ x: hub.right, y: hub.y }} to={{ x: provider.left, y: provider.y }} />;
      })}
    </svg>
  );
}

function Wire({ from, to, app, focus, markerId, translated = false }: {
  from: { x: number; y: number };
  to: { x: number; y: number };
  app: AppKind;
  focus: ClientFilterValue;
  markerId: string;
  translated?: boolean;
}) {
  const bend = Math.max(24, (to.x - from.x) * .5);
  return <path className={`asb-gateway-wire${translated ? " is-translation" : ""}${focus !== "all" && focus !== app ? " is-muted" : ""}`}
    d={`M${from.x} ${from.y} C${from.x + bend} ${from.y} ${to.x - bend} ${to.y} ${to.x} ${to.y}`}
    markerEnd={`url(#${markerId})`} />;
}

function useDeviceGeometry(sceneRef: RefObject<HTMLDivElement | null>, routes: GatewayRouteStatus[]) {
  const [geometry, setGeometry] = useState<Geometry>({ width: 0, height: 0, anchors: {} });
  useLayoutEffect(() => {
    const scene = sceneRef.current;
    if (!scene) return;
    const nodes = [...scene.querySelectorAll<HTMLElement>("[data-gateway-node]")];
    const measure = () => {
      const bounds = scene.getBoundingClientRect();
      const anchors: Record<string, Anchor> = {};
      for (const node of nodes) {
        const device = node.querySelector<SVGSVGElement>(".asb-gateway-device");
        const key = node.dataset.gatewayNode;
        if (!device || !key) continue;
        const rect = device.getBoundingClientRect();
        anchors[key] = {
          left: rect.left - bounds.left,
          right: rect.right - bounds.left,
          y: rect.top - bounds.top + rect.height * .6,
        };
      }
      setGeometry({ width: bounds.width, height: bounds.height, anchors });
    };
    let frame: number | null = null;
    const observer = new ResizeObserver(() => {
      if (frame !== null) window.cancelAnimationFrame(frame);
      frame = window.requestAnimationFrame(measure);
    });
    observer.observe(scene);
    nodes.forEach((node) => observer.observe(node));
    measure();
    return () => {
      observer.disconnect();
      if (frame !== null) window.cancelAnimationFrame(frame);
    };
  }, [sceneRef, routes]);
  return geometry;
}
