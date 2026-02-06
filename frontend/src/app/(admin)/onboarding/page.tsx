"use client"

import { useState } from "react"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
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
import { Check, ChevronRight, Upload } from "lucide-react"

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

export default function OnboardingPage() {
  const [step, setStep] = useState(1)
  const [completedSteps, setCompletedSteps] = useState<number[]>([])

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
      setCompletedSteps((prev) => [...prev, step])
      setStep((prev) => prev + 1)
    }
  }

  const handleBack = () => {
    setStep((prev) => prev - 1)
  }

  const onSubmit = (data: OnboardingValues) => {
    console.log("Form submitted:", data)
    // Here you would typically send data to backend
    alert("Onboarding data submitted successfully!")
  }

  const renderStepIndicator = () => (
    <div className="mb-8">
      <div className="flex justify-between items-center mb-4">
        {[1, 2, 3, 4].map((i) => (
          <div
            key={i}
            className={`flex flex-col items-center ${
              i <= step ? "text-primary" : "text-muted-foreground"
            }`}
          >
            <div
              className={`w-10 h-10 rounded-full flex items-center justify-center border-2 mb-2 ${
                i < step || completedSteps.includes(i)
                  ? "bg-primary text-primary-foreground border-primary"
                  : i === step
                  ? "border-primary text-primary"
                  : "border-muted text-muted-foreground"
              }`}
            >
              {i < step || completedSteps.includes(i) ? (
                <Check className="w-6 h-6" />
              ) : (
                i
              )}
            </div>
            <span className="text-sm font-medium">
              {i === 1 && "Company"}
              {i === 2 && "Branding"}
              {i === 3 && "API"}
              {i === 4 && "Review"}
            </span>
          </div>
        ))}
      </div>
      <div className="h-2 bg-secondary rounded-full overflow-hidden">
        <div
          className="h-full bg-primary transition-all duration-300 ease-in-out"
          style={{ width: `${((step - 1) / 3) * 100}%` }}
        />
      </div>
    </div>
  )

  return (
    <div className="container max-w-3xl mx-auto py-10">
      <div className="mb-8 text-center">
        <h1 className="text-3xl font-bold tracking-tight mb-2">
          Enterprise Onboarding
        </h1>
        <p className="text-muted-foreground">
          Complete your organization profile to get started with RampOS
        </p>
      </div>

      {renderStepIndicator()}

      <Card>
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)}>
            {step === 1 && (
              <CardContent className="space-y-4 pt-6">
                <div className="grid grid-cols-2 gap-4">
                  <FormField
                    control={form.control}
                    name="companyName"
                    render={({ field }) => (
                      <FormItem className="col-span-2">
                        <FormLabel>Company Name</FormLabel>
                        <FormControl>
                          <Input placeholder="Acme Corp" {...field} />
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
                          <Input placeholder="REG-123456" {...field} />
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
                          <Input placeholder="TAX-987654" {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={form.control}
                    name="address"
                    render={({ field }) => (
                      <FormItem className="col-span-2">
                        <FormLabel>Headquarters Address</FormLabel>
                        <FormControl>
                          <Input placeholder="123 Business Ave, Tech City" {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={form.control}
                    name="country"
                    render={({ field }) => (
                      <FormItem className="col-span-2">
                        <FormLabel>Country of Incorporation</FormLabel>
                        <FormControl>
                          <Input placeholder="United States" {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                </div>
              </CardContent>
            )}

            {step === 2 && (
              <CardContent className="space-y-6 pt-6">
                <div className="space-y-4">
                  <FormField
                    control={form.control}
                    name="brandColor"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Primary Brand Color</FormLabel>
                        <div className="flex items-center gap-4">
                          <FormControl>
                            <Input
                              type="color"
                              className="w-20 h-10 p-1 cursor-pointer"
                              {...field}
                            />
                          </FormControl>
                          <Input
                            {...field}
                            className="w-32 uppercase"
                            placeholder="#000000"
                          />
                        </div>
                        <FormDescription>
                          This color will be used for your customer-facing pages.
                        </FormDescription>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

                  <div className="space-y-2">
                    <FormLabel>Company Logo</FormLabel>
                    <div className="border-2 border-dashed rounded-lg p-8 flex flex-col items-center justify-center text-center cursor-pointer hover:bg-secondary/50 transition-colors">
                      <div className="w-12 h-12 rounded-full bg-secondary flex items-center justify-center mb-4">
                        <Upload className="w-6 h-6 text-muted-foreground" />
                      </div>
                      <p className="font-medium">Click to upload logo</p>
                      <p className="text-sm text-muted-foreground mt-1">
                        SVG, PNG, JPG (max 2MB)
                      </p>
                    </div>
                  </div>
                </div>
              </CardContent>
            )}

            {step === 3 && (
              <CardContent className="space-y-6 pt-6">
                <FormField
                  control={form.control}
                  name="environment"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Initial Environment</FormLabel>
                      <div className="grid grid-cols-2 gap-4">
                        <div
                          className={`border rounded-lg p-4 cursor-pointer hover:border-primary transition-all ${
                            field.value === "sandbox"
                              ? "border-primary bg-primary/5"
                              : ""
                          }`}
                          onClick={() => field.onChange("sandbox")}
                        >
                          <h3 className="font-semibold mb-1">Sandbox</h3>
                          <p className="text-sm text-muted-foreground">
                            Test environment with fake money and data.
                          </p>
                        </div>
                        <div
                          className={`border rounded-lg p-4 cursor-pointer hover:border-primary transition-all ${
                            field.value === "production"
                              ? "border-primary bg-primary/5"
                              : ""
                          }`}
                          onClick={() => field.onChange("production")}
                        >
                          <h3 className="font-semibold mb-1">Production</h3>
                          <p className="text-sm text-muted-foreground">
                            Live environment for real transactions.
                          </p>
                        </div>
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
                        <Input
                          placeholder="https://api.yourcompany.com/webhooks"
                          {...field}
                        />
                      </FormControl>
                      <FormDescription>
                        We'll send event notifications to this URL.
                      </FormDescription>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </CardContent>
            )}

            {step === 4 && (
              <CardContent className="pt-6">
                <div className="space-y-6">
                  <div className="bg-secondary/30 rounded-lg p-4 space-y-4">
                    <h3 className="font-semibold text-lg border-b pb-2">
                      Company Information
                    </h3>
                    <dl className="grid grid-cols-2 gap-4 text-sm">
                      <div>
                        <dt className="text-muted-foreground">Company Name</dt>
                        <dd className="font-medium">
                          {getValues("companyName")}
                        </dd>
                      </div>
                      <div>
                        <dt className="text-muted-foreground">Reg. Number</dt>
                        <dd className="font-medium">
                          {getValues("registrationNumber")}
                        </dd>
                      </div>
                      <div>
                        <dt className="text-muted-foreground">Tax ID</dt>
                        <dd className="font-medium">{getValues("taxId")}</dd>
                      </div>
                      <div>
                        <dt className="text-muted-foreground">Country</dt>
                        <dd className="font-medium">{getValues("country")}</dd>
                      </div>
                    </dl>
                  </div>

                  <div className="bg-secondary/30 rounded-lg p-4 space-y-4">
                    <h3 className="font-semibold text-lg border-b pb-2">
                      Configuration
                    </h3>
                    <dl className="grid grid-cols-2 gap-4 text-sm">
                      <div>
                        <dt className="text-muted-foreground">Environment</dt>
                        <dd className="font-medium capitalize">
                          {getValues("environment")}
                        </dd>
                      </div>
                      <div>
                        <dt className="text-muted-foreground">Brand Color</dt>
                        <dd className="font-medium flex items-center gap-2">
                          <div
                            className="w-4 h-4 rounded-full border"
                            style={{ backgroundColor: getValues("brandColor") }}
                          />
                          {getValues("brandColor")}
                        </dd>
                      </div>
                    </dl>
                  </div>
                </div>
              </CardContent>
            )}

            <CardFooter className="flex justify-between pt-6 border-t">
              <Button
                type="button"
                variant="outline"
                onClick={handleBack}
                disabled={step === 1}
              >
                Back
              </Button>
              {step < 4 ? (
                <Button type="button" onClick={handleNext}>
                  Next Step
                  <ChevronRight className="w-4 h-4 ml-2" />
                </Button>
              ) : (
                <Button type="submit">Complete Onboarding</Button>
              )}
            </CardFooter>
          </form>
        </Form>
      </Card>
    </div>
  )
}
