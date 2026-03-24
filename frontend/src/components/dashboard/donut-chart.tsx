'use client'

import React, { useMemo } from 'react'

interface DonutSegment {
  name: string
  value: number
  color: string
}

interface DonutChartProps {
  data: DonutSegment[]
  centerLabel: string
  centerValue: string
}

export function DonutChart({ data, centerLabel, centerValue }: DonutChartProps) {
  const total = useMemo(() => data.reduce((sum, item) => sum + item.value, 0), [data])
  
  // Calculate SVG stroke-dasharray properties for the donut segments
  const size = 200;
  const strokeWidth = 24;
  const radius = (size - strokeWidth) / 2;
  const circumference = radius * 2 * Math.PI;
  
  let currentOffset = 0;
  
  const segments = data.map((item, i) => {
    const percentage = item.value / total;
    const strokeLength = percentage * circumference;
    // Add small gap between segments, unless it's a very tiny segment
    const gap = percentage > 0.05 ? 2 : 0;
    
    const dashArray = `${strokeLength - gap} ${circumference}`;
    const dashOffset = -currentOffset;
    
    currentOffset += strokeLength;
    
    return {
      ...item,
      dashArray,
      dashOffset,
      percentage
    };
  });

  return (
    <div className="relative w-full h-[280px] flex flex-col items-center justify-center">
      <div className="relative w-[200px] h-[200px]">
        {/* Glow effect definitions */}
        <svg width="0" height="0" className="absolute">
          <defs>
            {data.map((d, i) => (
              <filter key={`glow-${i}`} id={`glow-${i}`} x="-20%" y="-20%" width="140%" height="140%">
                <feGaussianBlur stdDeviation="3" result="blur" />
                <feComposite in="SourceGraphic" in2="blur" operator="over" />
              </filter>
            ))}
          </defs>
        </svg>

        {/* The Donut SVG */}
        <svg 
          viewBox={`0 0 ${size} ${size}`} 
          className="w-full h-full transform -rotate-90 drop-shadow-2xl"
        >
          {segments.map((segment, i) => (
            <circle
              key={segment.name}
              cx={size / 2}
              cy={size / 2}
              r={radius}
              fill="transparent"
              stroke={segment.color}
              strokeWidth={strokeWidth}
              strokeDasharray={segment.dashArray}
              strokeDashoffset={segment.dashOffset}
              strokeLinecap="round"
              className="transition-all duration-700 ease-out hover:opacity-80 cursor-pointer origin-center"
              style={{ filter: `drop-shadow(0 0 8px ${segment.color}80)` }}
            />
          ))}
        </svg>
        
        {/* Center Content */}
        <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
          <span className="text-2xl font-bold text-white tabular-nums tracking-tight">
            {centerValue}
          </span>
          <span className="text-xs text-white/50 font-medium uppercase tracking-wider mt-1">
            {centerLabel}
          </span>
        </div>
      </div>

      {/* Legend below the chart */}
      <div className="flex flex-wrap justify-center gap-x-5 gap-y-2 mt-6">
        {data.map((d, i) => (
          <div key={i} className="flex items-center gap-2">
            <span 
              className="w-2.5 h-2.5 rounded-full shadow-lg" 
              style={{ 
                backgroundColor: d.color,
                boxShadow: `0 0 8px ${d.color}60`
              }} 
            />
            <span className="text-xs text-white/60 font-medium">{d.name}</span>
          </div>
        ))}
      </div>
    </div>
  )
}
