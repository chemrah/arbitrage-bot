import { useRef, useMemo, useEffect } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import { OrbitControls, Points, PointMaterial } from '@react-three/drei';
import * as THREE from 'three';
import { useAppStore } from '../lib/store';

interface Particle {
  position: [number, number, number];
  color: string;
  size: number;
  velocity: [number, number, number];
  life: number;
}

function MempoolParticles({ count = 300 }: { count?: number }) {
  const ref = useRef<THREE.Points>(null);
  const mempoolTxns = useAppStore((s) => s.mempoolTxns);

  const particles = useMemo(() => {
    const temp: Particle[] = [];
    for (let i = 0; i < count; i++) {
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);
      const r = 5 + Math.random() * 10;
      temp.push({
        position: [
          r * Math.sin(phi) * Math.cos(theta),
          r * Math.sin(phi) * Math.sin(theta),
          r * Math.cos(phi),
        ],
        color: Math.random() > 0.5 ? '#00e676' : '#448aff',
        size: 0.05 + Math.random() * 0.1,
        velocity: [
          (Math.random() - 0.5) * 0.01,
          (Math.random() - 0.5) * 0.01,
          (Math.random() - 0.5) * 0.01,
        ],
        life: Math.random(),
      });
    }
    return temp;
  }, [count]);

  const positions = useMemo(() => new Float32Array(particles.flatMap((p) => p.position)), [particles]);
  const colors = useMemo(
    () => new Float32Array(particles.flatMap((p) => {
      const c = new THREE.Color(p.color);
      return [c.r, c.g, c.b];
    })),
    [particles]
  );
  const sizes = useMemo(() => new Float32Array(particles.map((p) => p.size)), [particles]);

  useFrame((state) => {
    if (!ref.current) return;
    const elapsed = state.clock.getElapsedTime();
    const pos = ref.current.geometry.attributes.position.array as Float32Array;

    for (let i = 0; i < particles.length; i++) {
      const idx = i * 3;
      pos[idx] += Math.sin(elapsed * 0.5 + i) * 0.002;
      pos[idx + 1] += Math.cos(elapsed * 0.3 + i * 1.1) * 0.002;
      pos[idx + 2] += Math.sin(elapsed * 0.7 + i * 0.7) * 0.002;
    }
    ref.current.geometry.attributes.position.needsUpdate = true;
  });

  return (
    <points ref={ref}>
      <bufferGeometry>
        <bufferAttribute args={[positions, 3]} attach="attributes-position" />
        <bufferAttribute args={[colors, 3]} attach="attributes-color" />
        <bufferAttribute args={[sizes, 1]} attach="attributes-size" />
      </bufferGeometry>
      <pointsMaterial
        size={0.08}
        vertexColors
        transparent
        opacity={0.8}
        sizeAttenuation
        depthWrite={false}
        blending={THREE.AdditiveBlending}
      />
    </points>
  );
}

function Grid() {
  return (
    <gridHelper args={[20, 20, '#1a1a2e', '#1a1a2e']} position={[0, -4, 0]} />
  );
}

function SwarmNode({ position, color = '#448aff', size = 0.15 }: {
  position: [number, number, number];
  color?: string;
  size?: number;
}) {
  const meshRef = useRef<THREE.Mesh>(null);
  useFrame((state) => {
    if (!meshRef.current) return;
    meshRef.current.position.y += Math.sin(state.clock.elapsedTime * 2 + position[0]) * 0.002;
  });

  return (
    <mesh ref={meshRef} position={position}>
      <sphereGeometry args={[size, 16, 16]} />
      <meshBasicMaterial color={color} transparent opacity={0.6} />
    </mesh>
  );
}

export function Mempool3DVisualizer() {
  const opportunities = useAppStore((s) => s.opportunities);

  const activeNodes = useMemo(() => {
    return opportunities.slice(0, 20).map((op, i) => {
      const theta = (i / 20) * Math.PI * 2;
      const phi = Math.acos(2 * (i / 20) - 1);
      const r = 3 + Math.random() * 2;
      return {
        position: [
          r * Math.sin(phi) * Math.cos(theta),
          r * Math.sin(phi) * Math.sin(theta),
          r * Math.cos(phi),
        ] as [number, number, number],
        color: op.successProbability > 0.7 ? '#00e676' : '#ffd740',
        size: 0.1 + op.successProbability * 0.2,
      };
    });
  }, [opportunities]);

  return (
    <div className="card" style={{ height: 400 }}>
      <div className="card-header">
        <span className="card-title">Mempool 3D Visualizer</span>
        <div className="flex gap-2 text-[10px]">
          <span className="flex items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-[var(--accent-green)]" />
            High Prob
          </span>
          <span className="flex items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-[var(--accent-yellow)]" />
            Low Prob
          </span>
          <span className="text-[var(--text-secondary)]">
            {opportunities.length} active
          </span>
        </div>
      </div>
      <div style={{ height: 340, width: '100%' }}>
        <Canvas camera={{ position: [0, 0, 12], fov: 60 }}>
          <ambientLight intensity={0.3} />
          <MempoolParticles count={400} />
          <Grid />
          {activeNodes.map((node, i) => (
            <SwarmNode key={i} {...node} />
          ))}
          <OrbitControls enableZoom={true} enablePan={false} autoRotate autoRotateSpeed={1.5} />
        </Canvas>
      </div>
    </div>
  );
}
