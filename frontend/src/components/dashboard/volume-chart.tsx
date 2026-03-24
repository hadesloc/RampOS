'use client'

import React from 'react'
import { cn } from "@/lib/utils"

interface ChartData {
  name: string
  volume: number
}

interface VolumeChartProps {
  data: ChartData[]
  className?: string
}

export function VolumeChart({ data, className }: VolumeChartProps) {
  // Pure SVG implementation for guaranteed rendering (bypasses Recharts SSR/Sandbox issues)
  
  if (!data || data.length === 0) return null;

  const width = 800;
  const height = 240;
  const paddingX = 40;
  const paddingY = 20;

  const maxVolume = Math.max(...data.map(d => d.volume));
  const minVolume = 0; // scale to 0

  const getX = (index: number) => paddingX + (index * ((width - paddingX * 2) / (data.length - 1)));
  const getY = (value: number) => height - paddingY - ((value - minVolume) / (maxVolume - minVolume || 1)) * (height - paddingY * 2);

  // Generate SVG Path
  const linePath = data.map((d, i) => `${i === 0 ? 'M' : 'L'} ${getX(i)},${getY(d.volume)}`).join(' ');
  const areaPath = `${linePath} L ${getX(data.length - 1)},${height - paddingY} L ${getX(0)},${height - paddingY} Z`;

  return (
    <div className={cn("relative w-full h-[300px] flex flex-col justify-end", className)}>
      <div className="absolute inset-0 z-0">
        <svg viewBox={`0 0 ${width} ${height}`} className="w-full h-full preserve-3d" preserveAspectRatio="none">
          <defs>
            <linearGradient id="areaGradient" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#00FF87" stopOpacity="0.4" />
              <stop offset="50%" stopColor="#00FF87" stopOpacity="0.1" />
              <stop offset="100%" stopColor="#00FF87" stopOpacity="0" />
            </linearGradient>
            <filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
              <feGaussianBlur stdDeviation="4" result="blur" />
              <feComposite in="SourceGraphic" in2="blur" operator="over" />
            </filter>
          </defs>

          {/* Grid Lines */}
          {[0, 0.25, 0.5, 0.75, 1].map((ratio, i) => {
            const y = height - paddingY - (height - paddingY * 2) * ratio;
            return (
              <g key={`grid-${i}`}>
                <line x1={paddingX} y1={y} x2={width - paddingX} y2={y} stroke="rgba(255,255,255,0.04)" strokeDasharray="4 4" />
                <text x={paddingX - 10} y={y + 4} fill="rgba(255,255,255,0.3)" fontSize="10" textAnchor="end" className="font-mono">
                  {(maxVolume * ratio / 1000).toFixed(0)}B
                </text>
              </g>
            );
          })}

          {/* Data Path */}
          <path d={areaPath} fill="url(#areaGradient)" />
          <path d={linePath} fill="none" stroke="#00FF87" strokeWidth="2.5" filter="url(#glow)" />

          {/* Points & Labels (Only show some points to avoid clutter) */}
          {data.map((d, i) => {
            if (i % 2 !== 0 && i !== data.length - 1 && i !== 0) return null; // Skip some labels for clean look
            return (
              <g key={`point-${i}`} className="group cursor-default transition-all duration-300">
                <circle cx={getX(i)} cy={getY(d.volume)} r="4" fill="#111113" stroke="#00FF87" strokeWidth="2" className="opacity-0 group-hover:opacity-100 transition-opacity" />
                <circle cx={getX(i)} cy={getY(d.volume)} r="12" fill="transparent" /> {/* Hitbox */}
                <text x={getX(i)} y={height - 4} fill="rgba(255,255,255,0.4)" fontSize="10" textAnchor="middle" className="font-mono">
                  {d.name}
                </text>
              </g>
            );
          })}
        </svg>
      </div>
      
      {/* Decorative Gradient Overlay */}
      <div className="absolute inset-x-0 bottom-0 h-10 bg-gradient-to-t from-[#111113] to-transparent pointer-events-none" />
    </div>
  )
}
