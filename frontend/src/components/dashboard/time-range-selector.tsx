'use client'

interface TimeRangeSelectorProps {
  value: string
  onChange: (value: string) => void
  options?: string[]
}

export function TimeRangeSelector({ value, onChange, options = ['24H', '7D', '30D'] }: TimeRangeSelectorProps) {
  return (
    <div className="inline-flex items-center rounded-lg p-1" style={{ backgroundColor: 'rgba(255,255,255,0.04)' }}>
      {options.map((opt) => (
        <button
          key={opt}
          onClick={() => onChange(opt)}
          className={`px-3.5 py-1.5 text-xs font-medium rounded-md transition-all duration-200 ${
            value === opt
              ? 'bg-[#00FF87]/10 text-[#00FF87] shadow-[0_0_12px_rgba(0,255,135,0.1)]'
              : 'text-gray-500 hover:text-gray-300 hover:bg-white/[0.04]'
          }`}
        >
          {opt}
        </button>
      ))}
    </div>
  )
}
