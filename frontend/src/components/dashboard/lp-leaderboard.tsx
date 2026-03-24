'use client'

import { Trophy, Medal, Award } from 'lucide-react'

interface LPProvider {
  rank: number
  name: string
  volume: string
  successRate: number
}

interface LPLeaderboardProps {
  data: LPProvider[]
}

function RankBadge({ rank }: { rank: number }) {
  if (rank === 1) return <div className="w-7 h-7 rounded-full bg-amber-500/15 flex items-center justify-center"><Trophy className="w-3.5 h-3.5 text-amber-400" /></div>
  if (rank === 2) return <div className="w-7 h-7 rounded-full bg-gray-400/15 flex items-center justify-center"><Medal className="w-3.5 h-3.5 text-gray-300" /></div>
  if (rank === 3) return <div className="w-7 h-7 rounded-full bg-orange-600/15 flex items-center justify-center"><Award className="w-3.5 h-3.5 text-orange-400" /></div>
  return <div className="w-7 h-7 rounded-full bg-white/5 flex items-center justify-center text-xs font-bold text-gray-500">{rank}</div>
}

export function LPLeaderboard({ data }: LPLeaderboardProps) {
  return (
    <div className="space-y-1">
      {/* Header */}
      <div className="grid grid-cols-12 gap-2 px-4 py-2 text-[11px] uppercase tracking-wider text-gray-500 font-medium">
        <div className="col-span-1">#</div>
        <div className="col-span-5">Provider</div>
        <div className="col-span-3 text-right">Volume</div>
        <div className="col-span-3 text-right">Rate</div>
      </div>

      {/* Rows */}
      {data.map((lp) => (
        <div
          key={lp.rank}
          className="grid grid-cols-12 gap-2 items-center px-4 py-3 rounded-lg hover:bg-white/[0.03] transition-colors group cursor-default"
        >
          <div className="col-span-1">
            <RankBadge rank={lp.rank} />
          </div>
          <div className="col-span-5 font-medium text-sm text-white group-hover:text-[#00FF87] transition-colors">
            {lp.name}
          </div>
          <div className="col-span-3 text-right text-sm text-gray-400 tabular-nums font-medium">
            {lp.volume}
          </div>
          <div className="col-span-3 text-right">
            <span className={`text-sm tabular-nums font-medium ${
              lp.successRate >= 98 ? 'text-[#00FF87]' :
              lp.successRate >= 95 ? 'text-[#FFB800]' : 'text-[#FF4757]'
            }`}>
              {lp.successRate}%
            </span>
          </div>
        </div>
      ))}
    </div>
  )
}
