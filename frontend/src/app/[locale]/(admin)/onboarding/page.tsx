"use client"

import { useState } from "react"
import { useRouter } from "@/navigation"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { Check, ChevronRight, Loader2, Upload } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form"
import { Input } from "@/components/ui/input"
import { PageHeader, Panel, StatCard, StatGrid, StatusBadge } from "@/components/shared"
import { tenantsApi } from "@/lib/api"
import { useToast } from "@/components/ui/use-toast"
import { useTranslations } from "next-intl"
import { cn } from "@/lib/utils"

// Define validation schemas for each step
const companyInfoSchema = z.object({
  companyName: z.string().min(2, "Company name must be at least 2 characters"),
  registrationNumber: z.string().min(5, "Registration number is required"),
  taxId: z.string().min(5, "Tax ID is required"),
  address: z.string().min(10, "Address is required"),
  country: z.string().min(2, "Country is required"),
})

const brandingSchema = z.object({
  brandColor: z.string().regex(/^#([0-9A-F]{3}){1,2}$/i, "Invalid hex color code"),
  logoUrl: z.string().optional(),
})

const apiConfigSchema = z.object({
  webhookUrl: z.string().url("Must be a valid URL").optional().or(z.literal("")),
  environment: z.enum(["sandbox", "production"]),
})

// Combined schema for final submission
const onboardingSchema = z.object({
  ...companyInfoSchema.shape,
  ...brandingSchema.shape,
  ...apiConfigSchema.shape,
})

type OnboardingValues = z.infer<typeof onboardingSchema>

const steps = ["Company", "Branding", "API", "Review"]

const fieldClassName =
  "border-white/[0.08] bg-[#09090B]/70 text-foreground shadow-inner shadow-black/20 focus-visible:ring-[#00D4FF]/30"

export default function OnboardingPage() {
  const [step, setStep] = useState(1)
  const [completedSteps, setCompletedSteps] = useState<number[]>([])
  const [submitting, setSubmitting] = useState(false)
  const router = useRouter()
  const { toast } = useToast()
  const tCommon = useTranslations('Common')

  const form = useForm<OnboardingValues>({
    resolver: zodResolver(onboardingSchema),
    defaultValues: {
      companyName: "",
      registrationNumber: "",
      taxId: "",
      address: "",
      country: "",
      brandColor: "#0f172a",
      environment: "sandbox",
      webhookUrl: "",
    },
    mode: "onChange",
  })

  const { trigger, getValues } = form

  const handleNext = async () => {
    let isValid = false

    if (step === 1) {
      isValid = await trigger([
        "companyName",
        "registrationNumber",
        "taxId",
        "address",
        "country",
      ])
    } else if (step === 2) {
      isValid = await trigger(["brandColor"])
    } else if (step === 3) {
      isValid = await trigger(["environment", "webhookUrl"])
    }

    if (isValid) {
      setCompletedSteps((prev) => (prev.includes(step) ? prev : [...prev, step]))
      setStep((prev) => prev + 1)
    }
  }

  const handleBack = () => {
    setStep((prev) => prev - 1)
  }

  const onSubmit = async (data: OnboardingValues) => {
    setSubmitting(true)
    try {
      await tenantsApi.create({
        name: data.companyName,
        config: {
          registration_number: data.registrationNumber,
          tax_id: data.taxId,
          address: data.address,
          country: data.country,
          brand_color: data.brandColor,
          logo_url: data.logoUrl,
          environment: data.environment,
          webhook_url: data.webhookUrl,
        },
      })
      toast({
        title: tCommon('success'),
        description: "Organization onboarded successfully!",
      })
      router.push("/settings")
    } catch (err: any) {
      toast({
        variant: "destructive",
        title: "Onboarding Failed",
        description: err.message || "Failed to create tenant. Please try again.",
      })
    } finally {
      setSubmitting(false)
    }
  }

  const renderStepIndicator = () => (
    <Panel variant="glass" contentClassName="space-y-4">
      <div className="flex justify-between gap-2">
        {steps.map((label, index) => {
          const stepNumber = index + 1
          const complete = stepNumber < step || completedSteps.includes(stepNumber)
          const active = stepNumber === step

          return (
            <div
              key={label}
              className={cn(
                "flex min-w-0 flex-1 flex-col items-center text-center",
                active || complete ? "text-foreground" : "text-muted-foreground",
              )}
            >
              <div
                className={cn(
                  "mb-2 flex h-10 w-10 items-center justify-center rounded-full border-2 text-sm font-semibold transition-all",
                  complete && "border-[#00FF87] bg-[#00FF87]/15 text-[#00FF87] shadow-[0_0_22px_rgba(0,255,135,0.14)]",
                  active && !complete && "border-[#00D4FF] bg-[#00D4FF]/10 text-[#00D4FF]",
                  !active && !complete && "border-white/[0.08] bg-[#111113] text-muted-foreground",
                )}
              >
                {complete ? <Check className="h-5 w-5" /> : stepNumber}
              </div>
              <span className="truncate text-xs font-medium sm:text-sm">{label}</span>
            </div>
          )
        })}
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-white/[0.06]">
        <div
          className="h-full rounded-full bg-gradient-to-r from-[#00FF87] via-[#00D4FF] to-[#7B61FF] transition-all duration-300 ease-in-out"
          style={{ width: `${((step - 1) / 3) * 100}%` }}
        />
      </div>
    </Panel>
  )

  return (
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Enterprise Onboarding"
        description="Complete your organization profile to get started with RampOS."
      />

      <StatGrid>
        <StatCard title="Current step" value={`${step}/4`} subtitle={steps[step - 1]} accentColor="green" />
        <StatCard title="Completed" value={completedSteps.length} subtitle="Validated sections" accentColor="cyan" />
        <StatCard title="Environment" value={getValues("environment")} subtitle="Initial mode" accentColor="violet" />
        <StatCard title="Submission" value={submitting ? "Saving" : "Draft"} subtitle="Tenant creation" accentColor="amber" />
      </StatGrid>

      {renderStepIndicator()}

      <Panel className="mx-auto w-full max-w-4xl" contentClassName="p-0">
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)}>
            {step === 1 && (
              <div className="space-y-4 p-6">
                <SectionTitle title="Company information" description="Capture only the organization details required for tenant setup." />
                <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                  <FormField
                    control={form.control}
                    name="companyName"
                    render={({ field }) => (
                      <FormItem className="md:col-span-2">
                        <FormLabel>Company Name</FormLabel>
                        <FormControl>
                          <Input placeholder="Acme Corp" className={fieldClassName} {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={form.control}
                    name="registrationNumber"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Registration Number</FormLabel>
                        <FormControl>
                          <Input placeholder="REG-123456" className={fieldClassName} {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={form.control}
                    name="taxId"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Tax ID / VAT</FormLabel>
                        <FormControl>
                          <Input placeholder="TAX-987654" className={fieldClassName} {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={form.control}
                    name="address"
                    render={({ field }) => (
                      <FormItem className="md:col-span-2">
                        <FormLabel>Headquarters Address</FormLabel>
                        <FormControl>
                          <Input placeholder="123 Business Ave, Tech City" className={fieldClassName} {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={form.control}
                    name="country"
                    render={({ field }) => (
                      <FormItem className="md:col-span-2">
                        <FormLabel>Country of Incorporation</FormLabel>
                        <FormControl>
                          <Input placeholder="United States" className={fieldClassName} {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                </div>
              </div>
            )}

            {step === 2 && (
              <div className="space-y-6 p-6">
                <SectionTitle title="Branding" description="Apply customer-facing styling without changing operational data." />
                <FormField
                  control={form.control}
                  name="brandColor"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Primary Brand Color</FormLabel>
                      <div className="flex flex-wrap items-center gap-4">
                        <FormControl>
                          <Input type="color" className="h-11 w-20 cursor-pointer border-white/[0.08] bg-[#09090B] p-1" {...field} />
                        </FormControl>
                        <Input {...field} className={cn(fieldClassName, "w-36 uppercase")} placeholder="#000000" />
                        <div className="h-11 w-11 rounded-full border border-white/[0.12]" style={{ backgroundColor: field.value }} />
                      </div>
                      <FormDescription>This color will be used for your customer-facing pages.</FormDescription>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <div className="space-y-2">
                  <FormLabel>Company Logo</FormLabel>
                  <div className="flex cursor-pointer flex-col items-center justify-center rounded-xl border border-dashed border-[#7B61FF]/35 bg-[#7B61FF]/10 p-8 text-center transition-colors hover:bg-[#7B61FF]/15">
                    <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-[#00D4FF]/10">
                      <Upload className="h-6 w-6 text-[#00D4FF]" />
                    </div>
                    <p className="font-medium">Click to upload logo</p>
                    <p className="mt-1 text-sm text-muted-foreground">SVG, PNG, JPG (max 2MB)</p>
                  </div>
                </div>
              </div>
            )}

            {step === 3 && (
              <div className="space-y-6 p-6">
                <SectionTitle title="API configuration" description="Select the initial environment and optional webhook destination." />
                <FormField
                  control={form.control}
                  name="environment"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Initial Environment</FormLabel>
                      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                        {(["sandbox", "production"] as const).map((environment) => {
                          const selected = field.value === environment
                          return (
                            <button
                              key={environment}
                              type="button"
                              className={cn(
                                "rounded-xl border p-4 text-left transition-all",
                                selected
                                  ? "border-[#00FF87]/40 bg-[#00FF87]/10 shadow-[0_0_24px_rgba(0,255,135,0.1)]"
                                  : "border-white/[0.06] bg-[#09090B]/50 hover:border-[#00D4FF]/35 hover:bg-[#00D4FF]/5",
                              )}
                              onClick={() => field.onChange(environment)}
                            >
                              <div className="flex items-center justify-between gap-3">
                                <h3 className="font-semibold capitalize">{environment}</h3>
                                {selected ? <StatusBadge status="Selected" severity="success" /> : null}
                              </div>
                              <p className="mt-2 text-sm text-muted-foreground">
                                {environment === "sandbox"
                                  ? "Test environment with fake money and data."
                                  : "Live environment for real transactions."}
                              </p>
                            </button>
                          )
                        })}
                      </div>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                  control={form.control}
                  name="webhookUrl"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Webhook URL (Optional)</FormLabel>
                      <FormControl>
                        <Input placeholder="https://api.yourcompany.com/webhooks" className={fieldClassName} {...field} />
                      </FormControl>
                      <FormDescription>We&apos;ll send event notifications to this URL.</FormDescription>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </div>
            )}

            {step === 4 && (
              <div className="space-y-6 p-6">
                <SectionTitle title="Review" description="Confirm the tenant profile before creation." />
                <ReviewBlock
                  title="Company Information"
                  rows={[
                    ["Company Name", getValues("companyName")],
                    ["Reg. Number", getValues("registrationNumber")],
                    ["Tax ID", getValues("taxId")],
                    ["Country", getValues("country")],
                  ]}
                />

                <div className="rounded-xl border border-white/[0.06] bg-[#09090B]/50 p-4">
                  <h3 className="mb-4 text-lg font-semibold">Configuration</h3>
                  <dl className="grid grid-cols-1 gap-4 text-sm md:grid-cols-2">
                    <div>
                      <dt className="text-muted-foreground">Environment</dt>
                      <dd className="mt-1 font-medium capitalize">{getValues("environment")}</dd>
                    </div>
                    <div>
                      <dt className="text-muted-foreground">Brand Color</dt>
                      <dd className="mt-1 flex items-center gap-2 font-medium">
                        <div className="h-4 w-4 rounded-full border border-white/[0.16]" style={{ backgroundColor: getValues("brandColor") }} />
                        {getValues("brandColor")}
                      </dd>
                    </div>
                  </dl>
                </div>
              </div>
            )}

            <div className="flex justify-between border-t border-white/[0.06] p-6">
              <Button type="button" variant="outline" onClick={handleBack} disabled={step === 1}>
                Back
              </Button>
              {step < 4 ? (
                <Button type="button" onClick={handleNext} className="bg-[#00FF87] text-black hover:bg-[#00FF87]/90">
                  Next Step
                  <ChevronRight className="ml-2 h-4 w-4" />
                </Button>
              ) : (
                <Button type="submit" disabled={submitting} className="bg-[#00FF87] text-black hover:bg-[#00FF87]/90">
                  {submitting ? (
                    <>
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      Submitting...
                    </>
                  ) : (
                    "Complete Onboarding"
                  )}
                </Button>
              )}
            </div>
          </form>
        </Form>
      </Panel>
    </main>
  )
}

function SectionTitle({ title, description }: { title: string; description: string }) {
  return (
    <div>
      <h2 className="text-lg font-semibold tracking-tight">{title}</h2>
      <p className="text-sm text-muted-foreground">{description}</p>
    </div>
  )
}

function ReviewBlock({ title, rows }: { title: string; rows: Array<[string, string]> }) {
  return (
    <div className="rounded-xl border border-white/[0.06] bg-[#09090B]/50 p-4">
      <h3 className="mb-4 text-lg font-semibold">{title}</h3>
      <dl className="grid grid-cols-1 gap-4 text-sm md:grid-cols-2">
        {rows.map(([label, value]) => (
          <div key={label}>
            <dt className="text-muted-foreground">{label}</dt>
            <dd className="mt-1 font-medium">{value}</dd>
          </div>
        ))}
      </dl>
    </div>
  )
}
