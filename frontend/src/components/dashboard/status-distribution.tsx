'use client'

import React, { useEffect, useState } from 'react'

interface StatusData {
  name: string
  value: number
  color: string
}

interface StatusDistributionProps {
  data: StatusData[]
}

export function StatusDistribution({ data }: StatusDistributionProps) {
  const [mounted, setMounted] = useState(false)
  
  useEffect(() => {
    // Trigger animation shortly after mount
    const timer = setTimeout(() => setMounted(true), 100)
    return () => clearTimeout(timer)
  }, [])

  const total = data.reduce((sum, item) => sum + item.value, 0)
  const max = Math.max(...data.map(d => d.value))

  return (
    <div className="w-full h-[280px] flex flex-col justify-center gap-5 px-2">
      {data.map((item, i) => {
        // Calculate width relative to the maximum value to fill space well
        // But also show percentage relative to total
        const targetWidth = max > 0 ? (item.value / max) * 100 : 0;
        const percentage = total > 0 ? ((item.value / total) * 100).toFixed(1) : '0';

        return (
          <div key={item.name} className="flex flex-col gap-2">
            <div className="flex justify-between items-end text-sm">
              <span className="text-gray-300 font-medium">{item.name}</span>
              <div className="flex items-center gap-3">
                <span className="text-white font-bold tabular-nums">
                  {item.value.toLocaleString()}
                </span>
                <span className="text-gray-500 text-xs w-10 text-right">
                  {percentage}%
                </span>
              </div>
            </div>
            
            <div className="h-2.5 w-full bg-white/[0.03] rounded-full overflow-hidden relative shadow-inner">
              <div 
                className="absolute top-0 left-0 h-full rounded-full transition-all duration-1000 ease-out"
                style={{ 
                  width: mounted ? `${targetWidth}%` : '0%',
                  backgroundColor: item.color,
                  boxShadow: `0 0 10px ${item.color}80`
                }}
              />
            </div>
          </div>
        )
      })}
    </div>
  )
}
