"use client"

import * as React from "react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip, Legend } from "recharts"
import { ArrowUpRight, ArrowDownRight, Wallet } from "lucide-react"

// Mock data
const assets = [
  {
    symbol: "VND",
    name: "Vietnamese Dong",
    balance: 150000000,
    price: 1,
    change24h: 0,
    color: "#ef4444", // red-500
  },
  {
    symbol: "USDT",
    name: "Tether",
    balance: 5430.5,
    price: 25450,
    change24h: 0.15,
    color: "#22c55e", // green-500
  },
  {
    symbol: "ETH",
    name: "Ethereum",
    balance: 2.5,
    price: 85000000,
    change24h: -1.2,
    color: "#3b82f6", // blue-500
  },
  {
    symbol: "BTC",
    name: "Bitcoin",
    balance: 0.15,
    price: 1650000000,
    change24h: 2.5,
    color: "#f59e0b", // amber-500
  },
]

const formatVND = (value: number) => {
  return new Intl.NumberFormat("vi-VN", {
    style: "currency",
    currency: "VND",
  }).format(value)
}

const formatCrypto = (value: number, symbol: string) => {
  return `${value.toLocaleString("en-US", { maximumFractionDigits: 8 })} ${symbol}`
}

export default function AssetsPage() {
  const totalBalanceVND = assets.reduce((acc, asset) => acc + asset.balance * asset.price, 0)

  const pieData = assets.map(asset => ({
    name: asset.symbol,
    value: asset.balance * asset.price,
    color: asset.color
  })).filter(item => item.value > 0)

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold tracking-tight">Assets Overview</h1>
      </div>

      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-7">
        {/* Total Balance Card */}
        <Card className="col-span-4 lg:col-span-4">
          <CardHeader>
            <CardTitle>Total Balance</CardTitle>
            <CardDescription>
              Estimated value of all assets in VND
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex flex-col gap-2">
              <span className="text-4xl font-bold">
                {formatVND(totalBalanceVND)}
              </span>
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <span className="flex items-center text-green-500">
                  <ArrowUpRight className="mr-1 h-4 w-4" />
                  +2.5%
                </span>
                <span>vs last 24h</span>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Asset Allocation Chart */}
        <Card className="col-span-4 lg:col-span-3">
          <CardHeader>
            <CardTitle>Asset Allocation</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="h-[200px] w-full">
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={pieData}
                    cx="50%"
                    cy="50%"
                    innerRadius={60}
                    outerRadius={80}
                    paddingAngle={5}
                    dataKey="value"
                  >
                    {pieData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={entry.color} />
                    ))}
                  </Pie>
                  <Tooltip
                    formatter={(value: number) => formatVND(value)}
                    contentStyle={{ backgroundColor: 'hsl(var(--card))', borderColor: 'hsl(var(--border))', color: 'hsl(var(--foreground))' }}
                    itemStyle={{ color: 'hsl(var(--foreground))' }}
                  />
                  <Legend verticalAlign="bottom" height={36}/>
                </PieChart>
              </ResponsiveContainer>
            </div>
          </CardContent>
        </Card>
      </div>

      <h2 className="text-xl font-semibold mt-4">Your Assets</h2>
      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4">
        {assets.map((asset) => (
          <Card key={asset.symbol}>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">
                {asset.name}
              </CardTitle>
              <div className="h-8 w-8 rounded-full flex items-center justify-center" style={{ backgroundColor: `${asset.color}20`, color: asset.color }}>
                 <Wallet size={16} />
              </div>
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{formatCrypto(asset.balance, asset.symbol)}</div>
              <p className="text-xs text-muted-foreground mt-1">
                {formatVND(asset.balance * asset.price)}
              </p>
              <div className={`flex items-center text-xs mt-2 ${asset.change24h >= 0 ? 'text-green-500' : 'text-red-500'}`}>
                {asset.change24h >= 0 ? <ArrowUpRight className="h-3 w-3 mr-1" /> : <ArrowDownRight className="h-3 w-3 mr-1" />}
                {Math.abs(asset.change24h)}%
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  )
}
