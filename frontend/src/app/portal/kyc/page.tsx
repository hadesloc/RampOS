"use client"

import { useState } from "react"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Progress } from "@/components/ui/progress"

const kycSchema = z.object({
  firstName: z.string().min(2, "First name must be at least 2 characters"),
  lastName: z.string().min(2, "Last name must be at least 2 characters"),
  dob: z.string().refine((val) => !isNaN(Date.parse(val)), "Invalid date"),
  address: z.string().min(5, "Address must be at least 5 characters"),
  idDocument: z.any().optional(),
  selfie: z.any().optional(),
})

type KYCFormData = z.infer<typeof kycSchema>

export default function KYCPage() {
  const [step, setStep] = useState(1)
  const [progress, setProgress] = useState(25)

  const {
    register,
    handleSubmit,
    trigger,
    formState: { errors },
    watch
  } = useForm<KYCFormData>({
    resolver: zodResolver(kycSchema),
    mode: "onChange",
  })

  const formData = watch()

  const nextStep = async () => {
    let fieldsToValidate: (keyof KYCFormData)[] = []

    if (step === 1) {
      fieldsToValidate = ["firstName", "lastName", "dob", "address"]
    }

    const isValid = await trigger(fieldsToValidate)

    // We allow proceeding on file steps without strict validation for this placeholder
    if (isValid || step > 1) {
       setStep((prev) => Math.min(prev + 1, 4))
       setProgress((prev) => Math.min(prev + 25, 100))
    }
  }

  const prevStep = () => {
    setStep((prev) => Math.max(prev - 1, 1))
    setProgress((prev) => Math.max(prev - 25, 25))
  }

  const onSubmit = (data: KYCFormData) => {
    console.log("KYC Submission:", data)
    alert("KYC Submitted! Check console for data.")
  }

  return (
    <div className="container max-w-2xl py-10">
       <div className="mb-8 space-y-2">
         <h1 className="text-3xl font-bold">Identity Verification</h1>
         <p className="text-muted-foreground">Complete these steps to verify your account.</p>
         <Progress value={progress} className="mt-4" />
       </div>

       <Card>
         <form onSubmit={handleSubmit(onSubmit)}>
           {step === 1 && (
             <>
               <CardHeader>
                 <CardTitle>Personal Information</CardTitle>
                 <CardDescription>Enter your legal details as they appear on your ID.</CardDescription>
               </CardHeader>
               <CardContent className="space-y-4">
                 <div className="grid grid-cols-2 gap-4">
                   <div className="space-y-2">
                     <Label htmlFor="firstName">First Name</Label>
                     <Input id="firstName" {...register("firstName")} placeholder="John" />
                     {errors.firstName && <p className="text-sm text-red-500">{errors.firstName?.message as string}</p>}
                   </div>
                   <div className="space-y-2">
                     <Label htmlFor="lastName">Last Name</Label>
                     <Input id="lastName" {...register("lastName")} placeholder="Doe" />
                     {errors.lastName && <p className="text-sm text-red-500">{errors.lastName?.message as string}</p>}
                   </div>
                 </div>
                 <div className="space-y-2">
                   <Label htmlFor="dob">Date of Birth</Label>
                   <Input id="dob" type="date" {...register("dob")} />
                   {errors.dob && <p className="text-sm text-red-500">{errors.dob?.message as string}</p>}
                 </div>
                 <div className="space-y-2">
                   <Label htmlFor="address">Address</Label>
                   <Input id="address" {...register("address")} placeholder="123 Main St, City, Country" />
                   {errors.address && <p className="text-sm text-red-500">{errors.address?.message as string}</p>}
                 </div>
               </CardContent>
             </>
           )}

           {step === 2 && (
             <>
               <CardHeader>
                 <CardTitle>ID Document</CardTitle>
                 <CardDescription>Upload a clear picture of your government-issued ID.</CardDescription>
               </CardHeader>
               <CardContent className="space-y-4">
                 <div className="grid w-full max-w-sm items-center gap-1.5">
                   <Label htmlFor="idDocument">Upload ID (Passport/Driver's License)</Label>
                   <Input id="idDocument" type="file" {...register("idDocument")} />
                   <p className="text-sm text-muted-foreground">Supported formats: JPG, PNG, PDF</p>
                 </div>
               </CardContent>
             </>
           )}

           {step === 3 && (
             <>
               <CardHeader>
                 <CardTitle>Selfie Verification</CardTitle>
                 <CardDescription>Take a selfie to verify it's really you.</CardDescription>
               </CardHeader>
               <CardContent className="space-y-4">
                 <div className="grid w-full max-w-sm items-center gap-1.5">
                   <Label htmlFor="selfie">Upload Selfie</Label>
                   <Input id="selfie" type="file" accept="image/*" {...register("selfie")} />
                 </div>
               </CardContent>
             </>
           )}

           {step === 4 && (
             <>
               <CardHeader>
                 <CardTitle>Review & Submit</CardTitle>
                 <CardDescription>Please review your information before submitting.</CardDescription>
               </CardHeader>
               <CardContent className="space-y-4">
                 <div className="rounded-lg border p-4 space-y-2">
                    <div className="grid grid-cols-3 gap-2 text-sm">
                      <span className="font-medium text-muted-foreground">Name:</span>
                      <span className="col-span-2">{formData.firstName} {formData.lastName}</span>

                      <span className="font-medium text-muted-foreground">DOB:</span>
                      <span className="col-span-2">{formData.dob}</span>

                      <span className="font-medium text-muted-foreground">Address:</span>
                      <span className="col-span-2">{formData.address}</span>
                    </div>
                 </div>
                 <div className="text-sm text-muted-foreground">
                   By submitting, you agree to our Terms of Service and Privacy Policy.
                 </div>
               </CardContent>
             </>
           )}

           <CardFooter className="flex justify-between">
             {step > 1 ? (
               <Button type="button" variant="outline" onClick={prevStep}>Back</Button>
             ) : (
               <div /> // Spacer
             )}

             {step < 4 ? (
               <Button type="button" onClick={nextStep}>Next</Button>
             ) : (
               <Button type="submit">Submit Verification</Button>
             )}
           </CardFooter>
         </form>
       </Card>
    </div>
  )
}
