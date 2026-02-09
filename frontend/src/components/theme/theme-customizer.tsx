"use client";

import React, { useState, useEffect, useRef, useCallback } from "react";
import { Monitor, Moon, RotateCcw, Save, Sun, Undo } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle, CardFooter } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { useWhiteLabel, useThemeConfig } from "@/lib/theme/provider";
import { themePresets, getDefaultPreset } from "@/lib/theme/presets";
import { hslToHex, hexToHSL, type HSLColor, type ThemeColors } from "@/lib/theme/config";
import { toast } from "@/components/ui/use-toast";
import { cn } from "@/lib/utils";

// Simple Color Picker Component
function ColorPicker({
  label,
  color,
  onChange
}: {
  label: string;
  color: HSLColor;
  onChange: (color: HSLColor) => void;
}) {
  const hex = hslToHex(color);

  const handleHexChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newHex = e.target.value;
    // Simple validation for hex format
    if (/^#[0-9A-F]{6}$/i.test(newHex)) {
      onChange(hexToHSL(newHex));
    }
  };

  return (
    <div className="flex flex-col gap-2">
      <Label className="text-xs font-medium text-muted-foreground">{label}</Label>
      <div className="flex gap-2 items-center">
        <div className="relative w-full">
          <Input
            type="color"
            value={hex}
            onChange={(e) => onChange(hexToHSL(e.target.value))}
            className="h-9 w-full p-1 cursor-pointer absolute opacity-0"
          />
          <div className="flex w-full items-center gap-2 border rounded-md p-1 h-9 bg-background">
            <div
              className="h-6 w-6 rounded-sm border shadow-sm shrink-0"
              style={{ backgroundColor: hex }}
            />
            <span className="text-sm font-mono flex-1">{hex.toUpperCase()}</span>
          </div>
        </div>
      </div>
    </div>
  );
}

export function ThemeCustomizer() {
  const { theme, setTheme, updateColors, updateBrandName, updateLogo, resetToDefault } = useWhiteLabel();
  const [activeTab, setActiveTab] = useState("branding");
  const [previewMode, setPreviewMode] = useState<"light" | "dark">("light");

  // Local state for undo functionality
  const [history, setHistory] = useState([theme]);
  const [historyIndex, setHistoryIndex] = useState(0);
  const isUndoing = useRef(false);

  // Push current theme to history with 500ms debounce
  useEffect(() => {
    if (isUndoing.current) {
      isUndoing.current = false;
      return;
    }
    const timer = setTimeout(() => {
      setHistory(prev => {
        const truncated = prev.slice(0, historyIndex + 1);
        return [...truncated, theme];
      });
      setHistoryIndex(prev => prev + 1);
    }, 500);
    return () => clearTimeout(timer);
  }, [theme]);

  const handleUndo = useCallback(() => {
    if (historyIndex > 0) {
      isUndoing.current = true;
      const prevIndex = historyIndex - 1;
      setHistoryIndex(prevIndex);
      setTheme(history[prevIndex]);
      toast({
        title: "Undone",
        description: "Reverted to previous theme state.",
      });
    }
  }, [historyIndex, history, setTheme]);

  const handleApplyPreset = (presetId: string) => {
    const preset = themePresets.find(p => p.id === presetId);
    if (preset) {
      setTheme(preset);
      toast({
        title: "Preset Applied",
        description: `Applied ${preset.name} theme.`,
      });
    }
  };

  const handleSave = () => {
    // TODO: Persist to server API when endpoint is available
    localStorage.setItem("whitelabel-theme", JSON.stringify(theme));
    toast({
      title: "Theme Saved",
      description: "Your branding settings have been saved locally.",
    });
  };

  const handleReset = () => {
    resetToDefault();
    toast({
      title: "Theme Reset",
      description: "Restored default RampOS theme. Use Undo to revert.",
    });
  };

  const currentColors = previewMode === "light" ? theme.colors.light : theme.colors.dark;

  return (
    <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 h-full">
      {/* Editor Panel */}
      <div className="lg:col-span-4 flex flex-col gap-6 h-full overflow-y-auto pr-2">
        <div className="flex items-center justify-between">
          <h2 className="text-2xl font-bold tracking-tight">Theme Editor</h2>
          <div className="flex gap-2">
            <Button variant="outline" size="icon" onClick={handleUndo} disabled={historyIndex <= 0} title="Undo">
              <Undo className="h-4 w-4" />
            </Button>
            <Button variant="outline" size="icon" onClick={handleReset} title="Reset to Default">
              <RotateCcw className="h-4 w-4" />
            </Button>
            <Button onClick={handleSave} className="gap-2">
              <Save className="h-4 w-4" />
              Save
            </Button>
          </div>
        </div>

        <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="branding">Branding</TabsTrigger>
            <TabsTrigger value="colors">Colors</TabsTrigger>
            <TabsTrigger value="presets">Presets</TabsTrigger>
          </TabsList>

          {/* Branding Tab */}
          <TabsContent value="branding" className="space-y-6 mt-4">
            <Card>
              <CardHeader>
                <CardTitle>Brand Identity</CardTitle>
                <CardDescription>Configure your company name and logos.</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="brand-name">Brand Name</Label>
                  <Input
                    id="brand-name"
                    value={theme.brandName}
                    onChange={(e) => updateBrandName(e.target.value)}
                    placeholder="Acme Corp"
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="logo-light">Light Mode Logo URL</Label>
                  <Input
                    id="logo-light"
                    value={theme.logo.light}
                    onChange={(e) => updateLogo({ light: e.target.value })}
                    placeholder="/logo.svg"
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="logo-dark">Dark Mode Logo URL</Label>
                  <Input
                    id="logo-dark"
                    value={theme.logo.dark}
                    onChange={(e) => updateLogo({ dark: e.target.value })}
                    placeholder="/logo-dark.svg"
                  />
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Typography</CardTitle>
                <CardDescription>Customize font settings.</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="space-y-2">
                  <Label>Radius</Label>
                  <Select
                    value={theme.borderRadius.md}
                    onValueChange={(val) => setTheme({
                      ...theme,
                      borderRadius: {
                        ...theme.borderRadius,
                        md: val,
                        sm: `calc(${val} - 2px)`,
                        lg: `calc(${val} + 2px)`,
                      }
                    })}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select radius" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="0rem">None (0px)</SelectItem>
                      <SelectItem value="0.25rem">Small (4px)</SelectItem>
                      <SelectItem value="0.5rem">Medium (8px)</SelectItem>
                      <SelectItem value="0.75rem">Large (12px)</SelectItem>
                      <SelectItem value="1rem">Full (16px)</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          {/* Colors Tab */}
          <TabsContent value="colors" className="space-y-6 mt-4">
            <div className="flex items-center justify-between p-2 bg-muted rounded-md mb-4">
              <span className="text-sm font-medium ml-2">Editing Mode:</span>
              <div className="flex bg-background rounded-md p-1 border">
                <Button
                  variant={previewMode === "light" ? "secondary" : "ghost"}
                  size="sm"
                  onClick={() => setPreviewMode("light")}
                  className="gap-2 h-7"
                >
                  <Sun className="h-3 w-3" /> Light
                </Button>
                <Button
                  variant={previewMode === "dark" ? "secondary" : "ghost"}
                  size="sm"
                  onClick={() => setPreviewMode("dark")}
                  className="gap-2 h-7"
                >
                  <Moon className="h-3 w-3" /> Dark
                </Button>
              </div>
            </div>

            <Card>
              <CardHeader>
                <CardTitle>Brand Colors</CardTitle>
              </CardHeader>
              <CardContent className="grid grid-cols-2 gap-4">
                <ColorPicker
                  label="Primary"
                  color={currentColors.primary}
                  onChange={(c) => updateColors(previewMode, { primary: c })}
                />
                <ColorPicker
                  label="Foreground"
                  color={currentColors.primaryForeground}
                  onChange={(c) => updateColors(previewMode, { primaryForeground: c })}
                />
                <ColorPicker
                  label="Secondary"
                  color={currentColors.secondary}
                  onChange={(c) => updateColors(previewMode, { secondary: c })}
                />
                <ColorPicker
                  label="Accent"
                  color={currentColors.accent}
                  onChange={(c) => updateColors(previewMode, { accent: c })}
                />
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>UI Colors</CardTitle>
              </CardHeader>
              <CardContent className="grid grid-cols-2 gap-4">
                <ColorPicker
                  label="Background"
                  color={currentColors.background}
                  onChange={(c) => updateColors(previewMode, { background: c })}
                />
                <ColorPicker
                  label="Card Background"
                  color={currentColors.card}
                  onChange={(c) => updateColors(previewMode, { card: c })}
                />
                <ColorPicker
                  label="Border"
                  color={currentColors.border}
                  onChange={(c) => updateColors(previewMode, { border: c })}
                />
                <ColorPicker
                  label="Input"
                  color={currentColors.input}
                  onChange={(c) => updateColors(previewMode, { input: c })}
                />
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Status Colors</CardTitle>
              </CardHeader>
              <CardContent className="grid grid-cols-2 gap-4">
                <ColorPicker
                  label="Success"
                  color={currentColors.success}
                  onChange={(c) => updateColors(previewMode, { success: c })}
                />
                <ColorPicker
                  label="Warning"
                  color={currentColors.warning}
                  onChange={(c) => updateColors(previewMode, { warning: c })}
                />
                <ColorPicker
                  label="Destructive"
                  color={currentColors.destructive}
                  onChange={(c) => updateColors(previewMode, { destructive: c })}
                />
                <ColorPicker
                  label="Info"
                  color={currentColors.info}
                  onChange={(c) => updateColors(previewMode, { info: c })}
                />
              </CardContent>
            </Card>
          </TabsContent>

          {/* Presets Tab */}
          <TabsContent value="presets" className="mt-4">
            <div className="grid grid-cols-1 gap-4">
              {themePresets.map((preset) => (
                <Card
                  key={preset.id}
                  className={cn(
                    "cursor-pointer hover:border-primary transition-all overflow-hidden",
                    theme.id === preset.id ? "border-primary ring-2 ring-primary/20" : ""
                  )}
                  onClick={() => handleApplyPreset(preset.id)}
                >
                  <div className="h-24 w-full flex">
                    <div
                      className="w-1/3 h-full"
                      style={{ backgroundColor: hslToHex(preset.colors.light.primary) }}
                    />
                    <div
                      className="w-1/3 h-full"
                      style={{ backgroundColor: hslToHex(preset.colors.light.secondary) }}
                    />
                    <div
                      className="w-1/3 h-full"
                      style={{ backgroundColor: hslToHex(preset.colors.light.accent) }}
                    />
                  </div>
                  <CardHeader className="p-4">
                    <CardTitle className="text-base">{preset.name}</CardTitle>
                    <CardDescription className="text-xs">{preset.description}</CardDescription>
                  </CardHeader>
                </Card>
              ))}
            </div>
          </TabsContent>
        </Tabs>
      </div>

      {/* Live Preview Panel */}
      <div className="lg:col-span-8 bg-muted/20 rounded-xl border overflow-hidden flex flex-col h-full">
        <div className="bg-muted border-b p-3 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Monitor className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium text-muted-foreground">Live Preview</span>
          </div>
          <div className="flex gap-2">
            <Button
              variant={previewMode === "light" ? "secondary" : "ghost"}
              size="icon"
              className="h-8 w-8"
              onClick={() => setPreviewMode("light")}
              aria-label="Switch to light mode"
            >
              <Sun className="h-4 w-4" />
            </Button>
            <Button
              variant={previewMode === "dark" ? "secondary" : "ghost"}
              size="icon"
              className="h-8 w-8"
              onClick={() => setPreviewMode("dark")}
              aria-label="Switch to dark mode"
            >
              <Moon className="h-4 w-4" />
            </Button>
          </div>
        </div>

        <div className={cn(
          "flex-1 overflow-auto p-8 transition-colors duration-300",
          previewMode === "dark" ? "bg-slate-950" : "bg-slate-50"
        )}>
          {/* This preview area mimics the actual app UI structure */}
          <div className={cn(
            "max-w-4xl mx-auto rounded-lg shadow-xl overflow-hidden border transition-all duration-300",
            previewMode === "dark" ? "dark" : ""
          )}>
            <div className="bg-background text-foreground flex h-[600px] flex-col md:flex-row">
              {/* Fake Sidebar */}
              <div className="w-64 border-r bg-card p-4 flex flex-col gap-4">
                <div className="h-8 flex items-center gap-2 font-bold text-xl text-primary px-2">
                  <div className="h-6 w-6 rounded bg-primary" />
                  {theme.brandName}
                </div>
                <div className="space-y-1">
                  {['Dashboard', 'Transactions', 'Wallets', 'Settings'].map((item, i) => (
                    <div
                      key={item}
                      className={cn(
                        "h-9 rounded-md flex items-center px-3 text-sm font-medium",
                        i === 0 ? "bg-primary/10 text-primary" : "text-muted-foreground hover:bg-muted"
                      )}
                    >
                      {item}
                    </div>
                  ))}
                </div>
              </div>

              {/* Fake Content */}
              <div className="flex-1 flex flex-col bg-muted/10">
                <header className="h-14 border-b bg-background px-6 flex items-center justify-between">
                  <span className="font-semibold">Dashboard Overview</span>
                  <div className="h-8 w-8 rounded-full bg-primary/20" />
                </header>
                <main className="p-6 space-y-6">
                  <div className="grid grid-cols-3 gap-4">
                    {[
                      { label: "Total Revenue", value: "$45,231.89", trend: "+20.1%", trendUp: true },
                      { label: "Active Users", value: "+2350", trend: "+180.1%", trendUp: true },
                      { label: "Failed Txns", value: "12", trend: "-4.3%", trendUp: false },
                    ].map((stat, i) => (
                      <Card key={i}>
                        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                          <CardTitle className="text-sm font-medium">
                            {stat.label}
                          </CardTitle>
                        </CardHeader>
                        <CardContent>
                          <div className="text-2xl font-bold">{stat.value}</div>
                          <p className={cn(
                            "text-xs",
                            stat.trendUp ? "text-success" : "text-destructive"
                          )}>
                            {stat.trend} from last month
                          </p>
                        </CardContent>
                      </Card>
                    ))}
                  </div>

                  <div className="grid grid-cols-2 gap-4">
                    <Card className="col-span-1">
                      <CardHeader>
                        <CardTitle>Recent Activity</CardTitle>
                      </CardHeader>
                      <CardContent className="space-y-4">
                        {[1, 2, 3].map((i) => (
                          <div key={i} className="flex items-center gap-4">
                            <div className="h-9 w-9 rounded-full bg-secondary flex items-center justify-center">
                              TX
                            </div>
                            <div className="flex-1 space-y-1">
                              <p className="text-sm font-medium leading-none">Payment Received</p>
                              <p className="text-xs text-muted-foreground">User #{1000 + i}</p>
                            </div>
                            <div className="font-medium">+$250.00</div>
                          </div>
                        ))}
                      </CardContent>
                    </Card>

                    <Card className="col-span-1">
                      <CardHeader>
                        <CardTitle>UI Components</CardTitle>
                      </CardHeader>
                      <CardContent className="space-y-4">
                        <div className="flex flex-wrap gap-2">
                          <Button>Primary</Button>
                          <Button variant="secondary">Secondary</Button>
                          <Button variant="outline">Outline</Button>
                          <Button variant="destructive">Destructive</Button>
                        </div>
                        <div className="flex gap-2">
                           <Input placeholder="Input field..." />
                           <Button size="icon" variant="ghost"><Monitor className="h-4 w-4" /></Button>
                        </div>
                        <div className="flex gap-2 items-center">
                          <div className="h-2 w-full bg-secondary rounded-full overflow-hidden">
                            <div className="h-full w-[60%] bg-primary" />
                          </div>
                        </div>
                      </CardContent>
                    </Card>
                  </div>
                </main>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
