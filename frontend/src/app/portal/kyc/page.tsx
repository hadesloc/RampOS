"use client";

import { useState, useEffect, useCallback } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Progress } from "@/components/ui/progress";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useAuth } from "@/contexts/auth-context";
import { kycApi, KYCStatus } from "@/lib/portal-api";
import { useRouter } from "next/navigation";
import {
  Loader2,
  AlertCircle,
  CheckCircle2,
  Clock,
  XCircle,
  Upload,
  Camera,
} from "lucide-react";

const kycSchema = z.object({
  firstName: z.string().min(2, "First name must be at least 2 characters"),
  lastName: z.string().min(2, "Last name must be at least 2 characters"),
  dob: z.string().refine((val) => !isNaN(Date.parse(val)), "Invalid date"),
  address: z.string().min(5, "Address must be at least 5 characters"),
  idDocumentType: z.enum(["PASSPORT", "DRIVERS_LICENSE", "NATIONAL_ID"]),
});

type KYCFormData = z.infer<typeof kycSchema>;

type UploadedDocs = {
  idFront: File | null;
  idBack: File | null;
  selfie: File | null;
};

export default function KYCPage() {
  const [step, setStep] = useState(1);
  const [progress, setProgress] = useState(25);
  const [kycStatus, setKycStatus] = useState<KYCStatus | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isLoadingStatus, setIsLoadingStatus] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [uploadedDocs, setUploadedDocs] = useState<UploadedDocs>({
    idFront: null,
    idBack: null,
    selfie: null,
  });
  const [uploadProgress, setUploadProgress] = useState<Record<string, boolean>>(
    {}
  );

  const { user, isAuthenticated, isLoading: authLoading } = useAuth();
  const router = useRouter();

  const {
    register,
    handleSubmit,
    trigger,
    formState: { errors },
    watch,
    setValue,
  } = useForm<KYCFormData>({
    resolver: zodResolver(kycSchema),
    mode: "onChange",
  });

  const formData = watch();

  // Redirect if not authenticated
  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push("/portal/login");
    }
  }, [authLoading, isAuthenticated, router]);

  // Fetch KYC status
  const fetchKycStatus = useCallback(async () => {
    try {
      const status = await kycApi.getStatus();
      setKycStatus(status);
    } catch (err) {
      console.error("Failed to fetch KYC status:", err);
    } finally {
      setIsLoadingStatus(false);
    }
  }, []);

  useEffect(() => {
    if (isAuthenticated) {
      fetchKycStatus();
    }
  }, [isAuthenticated, fetchKycStatus]);

  const nextStep = async () => {
    let fieldsToValidate: (keyof KYCFormData)[] = [];

    if (step === 1) {
      fieldsToValidate = ["firstName", "lastName", "dob", "address", "idDocumentType"];
    }

    const isValid = await trigger(fieldsToValidate);

    if (isValid || step > 1) {
      setStep((prev) => Math.min(prev + 1, 4));
      setProgress((prev) => Math.min(prev + 25, 100));
    }
  };

  const prevStep = () => {
    setStep((prev) => Math.max(prev - 1, 1));
    setProgress((prev) => Math.max(prev - 25, 25));
  };

  const handleFileUpload = async (
    type: "idFront" | "idBack" | "selfie",
    file: File
  ) => {
    setUploadedDocs((prev) => ({ ...prev, [type]: file }));
    setUploadProgress((prev) => ({ ...prev, [type]: true }));

    try {
      const docType =
        type === "idFront"
          ? "ID_FRONT"
          : type === "idBack"
          ? "ID_BACK"
          : "SELFIE";
      await kycApi.uploadDocument(docType, file);
    } catch (err) {
      console.error("Failed to upload document:", err);
      setError(`Failed to upload ${type}. Please try again.`);
      setUploadedDocs((prev) => ({ ...prev, [type]: null }));
    } finally {
      setUploadProgress((prev) => ({ ...prev, [type]: false }));
    }
  };

  const onSubmit = async (data: KYCFormData) => {
    setIsSubmitting(true);
    setError(null);

    try {
      await kycApi.submit({
        firstName: data.firstName,
        lastName: data.lastName,
        dateOfBirth: data.dob,
        address: data.address,
        idDocumentType: data.idDocumentType,
      });

      // Refresh status
      await fetchKycStatus();
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to submit KYC. Please try again."
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  // Show loading state
  if (authLoading || isLoadingStatus) {
    return (
      <div className="container max-w-2xl py-10">
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      </div>
    );
  }

  // Show KYC status if already submitted
  if (kycStatus && kycStatus.status !== "NONE") {
    return (
      <div className="container max-w-2xl py-10">
        <div className="mb-8 space-y-2">
          <h1 className="text-3xl font-bold">Identity Verification</h1>
          <p className="text-muted-foreground">
            Your verification status
          </p>
        </div>

        <Card>
          <CardContent className="pt-6">
            {kycStatus.status === "PENDING" && (
              <div className="flex flex-col items-center text-center space-y-4 py-8">
                <div className="rounded-full bg-yellow-100 p-4 dark:bg-yellow-900/30">
                  <Clock className="h-12 w-12 text-yellow-600 dark:text-yellow-400" />
                </div>
                <div className="space-y-2">
                  <h2 className="text-xl font-semibold">Verification In Progress</h2>
                  <p className="text-muted-foreground max-w-md">
                    Your documents are being reviewed. This usually takes 1-2 business
                    days. We will notify you once the review is complete.
                  </p>
                </div>
                {kycStatus.submittedAt && (
                  <p className="text-sm text-muted-foreground">
                    Submitted on{" "}
                    {new Date(kycStatus.submittedAt).toLocaleDateString()}
                  </p>
                )}
              </div>
            )}

            {kycStatus.status === "VERIFIED" && (
              <div className="flex flex-col items-center text-center space-y-4 py-8">
                <div className="rounded-full bg-green-100 p-4 dark:bg-green-900/30">
                  <CheckCircle2 className="h-12 w-12 text-green-600 dark:text-green-400" />
                </div>
                <div className="space-y-2">
                  <h2 className="text-xl font-semibold">Verified</h2>
                  <p className="text-muted-foreground max-w-md">
                    Your identity has been verified. You now have full access to
                    all platform features.
                  </p>
                </div>
                <div className="rounded-lg bg-muted p-4">
                  <p className="text-sm">
                    <span className="text-muted-foreground">KYC Tier:</span>{" "}
                    <span className="font-medium">Level {kycStatus.tier}</span>
                  </p>
                </div>
              </div>
            )}

            {kycStatus.status === "REJECTED" && (
              <div className="flex flex-col items-center text-center space-y-4 py-8">
                <div className="rounded-full bg-red-100 p-4 dark:bg-red-900/30">
                  <XCircle className="h-12 w-12 text-red-600 dark:text-red-400" />
                </div>
                <div className="space-y-2">
                  <h2 className="text-xl font-semibold">Verification Failed</h2>
                  <p className="text-muted-foreground max-w-md">
                    Unfortunately, we could not verify your identity.
                  </p>
                </div>
                {kycStatus.rejectionReason && (
                  <Alert variant="destructive" className="max-w-md">
                    <AlertCircle className="h-4 w-4" />
                    <AlertDescription>{kycStatus.rejectionReason}</AlertDescription>
                  </Alert>
                )}
                <Button
                  onClick={() => {
                    setKycStatus({ ...kycStatus, status: "NONE" });
                    setStep(1);
                    setProgress(25);
                  }}
                >
                  Try Again
                </Button>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="container max-w-2xl py-10">
      <div className="mb-8 space-y-2">
        <h1 className="text-3xl font-bold">Identity Verification</h1>
        <p className="text-muted-foreground">
          Complete these steps to verify your account.
        </p>
        <Progress value={progress} className="mt-4" />
      </div>

      {error && (
        <Alert variant="destructive" className="mb-6">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <Card>
        <form onSubmit={handleSubmit(onSubmit)}>
          {step === 1 && (
            <>
              <CardHeader>
                <CardTitle>Personal Information</CardTitle>
                <CardDescription>
                  Enter your legal details as they appear on your ID.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label htmlFor="firstName">First Name</Label>
                    <Input
                      id="firstName"
                      {...register("firstName")}
                      placeholder="John"
                    />
                    {errors.firstName && (
                      <p className="text-sm text-red-500">
                        {errors.firstName?.message as string}
                      </p>
                    )}
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="lastName">Last Name</Label>
                    <Input
                      id="lastName"
                      {...register("lastName")}
                      placeholder="Doe"
                    />
                    {errors.lastName && (
                      <p className="text-sm text-red-500">
                        {errors.lastName?.message as string}
                      </p>
                    )}
                  </div>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="dob">Date of Birth</Label>
                  <Input id="dob" type="date" {...register("dob")} />
                  {errors.dob && (
                    <p className="text-sm text-red-500">
                      {errors.dob?.message as string}
                    </p>
                  )}
                </div>
                <div className="space-y-2">
                  <Label htmlFor="address">Address</Label>
                  <Input
                    id="address"
                    {...register("address")}
                    placeholder="123 Main St, City, Country"
                  />
                  {errors.address && (
                    <p className="text-sm text-red-500">
                      {errors.address?.message as string}
                    </p>
                  )}
                </div>
                <div className="space-y-2">
                  <Label htmlFor="idDocumentType">ID Document Type</Label>
                  <Select
                    onValueChange={(value) =>
                      setValue(
                        "idDocumentType",
                        value as "PASSPORT" | "DRIVERS_LICENSE" | "NATIONAL_ID"
                      )
                    }
                    defaultValue={formData.idDocumentType}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select document type" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="PASSPORT">Passport</SelectItem>
                      <SelectItem value="DRIVERS_LICENSE">
                        Driver's License
                      </SelectItem>
                      <SelectItem value="NATIONAL_ID">National ID Card</SelectItem>
                    </SelectContent>
                  </Select>
                  {errors.idDocumentType && (
                    <p className="text-sm text-red-500">
                      {errors.idDocumentType?.message as string}
                    </p>
                  )}
                </div>
              </CardContent>
            </>
          )}

          {step === 2 && (
            <>
              <CardHeader>
                <CardTitle>ID Document</CardTitle>
                <CardDescription>
                  Upload clear pictures of your {formData.idDocumentType?.toLowerCase().replace("_", " ") || "ID"}.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-6">
                <div className="space-y-4">
                  <div className="space-y-2">
                    <Label>Front of ID</Label>
                    <div className="border-2 border-dashed rounded-lg p-6 text-center">
                      {uploadedDocs.idFront ? (
                        <div className="flex items-center justify-center gap-2 text-green-600">
                          <CheckCircle2 className="h-5 w-5" />
                          <span>{uploadedDocs.idFront.name}</span>
                        </div>
                      ) : uploadProgress.idFront ? (
                        <div className="flex items-center justify-center gap-2">
                          <Loader2 className="h-5 w-5 animate-spin" />
                          <span>Uploading...</span>
                        </div>
                      ) : (
                        <label className="cursor-pointer">
                          <Upload className="h-8 w-8 mx-auto mb-2 text-muted-foreground" />
                          <span className="text-sm text-muted-foreground">
                            Click to upload front of ID
                          </span>
                          <input
                            type="file"
                            className="hidden"
                            accept="image/*,.pdf"
                            onChange={(e) => {
                              const file = e.target.files?.[0];
                              if (file) handleFileUpload("idFront", file);
                            }}
                          />
                        </label>
                      )}
                    </div>
                  </div>

                  <div className="space-y-2">
                    <Label>Back of ID (if applicable)</Label>
                    <div className="border-2 border-dashed rounded-lg p-6 text-center">
                      {uploadedDocs.idBack ? (
                        <div className="flex items-center justify-center gap-2 text-green-600">
                          <CheckCircle2 className="h-5 w-5" />
                          <span>{uploadedDocs.idBack.name}</span>
                        </div>
                      ) : uploadProgress.idBack ? (
                        <div className="flex items-center justify-center gap-2">
                          <Loader2 className="h-5 w-5 animate-spin" />
                          <span>Uploading...</span>
                        </div>
                      ) : (
                        <label className="cursor-pointer">
                          <Upload className="h-8 w-8 mx-auto mb-2 text-muted-foreground" />
                          <span className="text-sm text-muted-foreground">
                            Click to upload back of ID
                          </span>
                          <input
                            type="file"
                            className="hidden"
                            accept="image/*,.pdf"
                            onChange={(e) => {
                              const file = e.target.files?.[0];
                              if (file) handleFileUpload("idBack", file);
                            }}
                          />
                        </label>
                      )}
                    </div>
                  </div>
                </div>
                <p className="text-sm text-muted-foreground">
                  Supported formats: JPG, PNG, PDF. Max file size: 10MB
                </p>
              </CardContent>
            </>
          )}

          {step === 3 && (
            <>
              <CardHeader>
                <CardTitle>Selfie Verification</CardTitle>
                <CardDescription>
                  Take a selfie to verify it is really you.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="border-2 border-dashed rounded-lg p-8 text-center">
                  {uploadedDocs.selfie ? (
                    <div className="flex items-center justify-center gap-2 text-green-600">
                      <CheckCircle2 className="h-5 w-5" />
                      <span>{uploadedDocs.selfie.name}</span>
                    </div>
                  ) : uploadProgress.selfie ? (
                    <div className="flex items-center justify-center gap-2">
                      <Loader2 className="h-5 w-5 animate-spin" />
                      <span>Uploading...</span>
                    </div>
                  ) : (
                    <label className="cursor-pointer">
                      <Camera className="h-12 w-12 mx-auto mb-2 text-muted-foreground" />
                      <span className="text-sm text-muted-foreground block">
                        Click to take or upload a selfie
                      </span>
                      <span className="text-xs text-muted-foreground block mt-1">
                        Make sure your face is clearly visible
                      </span>
                      <input
                        type="file"
                        className="hidden"
                        accept="image/*"
                        capture="user"
                        onChange={(e) => {
                          const file = e.target.files?.[0];
                          if (file) handleFileUpload("selfie", file);
                        }}
                      />
                    </label>
                  )}
                </div>
              </CardContent>
            </>
          )}

          {step === 4 && (
            <>
              <CardHeader>
                <CardTitle>Review & Submit</CardTitle>
                <CardDescription>
                  Please review your information before submitting.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="rounded-lg border p-4 space-y-2">
                  <div className="grid grid-cols-3 gap-2 text-sm">
                    <span className="font-medium text-muted-foreground">
                      Name:
                    </span>
                    <span className="col-span-2">
                      {formData.firstName} {formData.lastName}
                    </span>

                    <span className="font-medium text-muted-foreground">
                      DOB:
                    </span>
                    <span className="col-span-2">{formData.dob}</span>

                    <span className="font-medium text-muted-foreground">
                      Address:
                    </span>
                    <span className="col-span-2">{formData.address}</span>

                    <span className="font-medium text-muted-foreground">
                      Document Type:
                    </span>
                    <span className="col-span-2">
                      {formData.idDocumentType?.replace("_", " ")}
                    </span>
                  </div>
                </div>

                <div className="rounded-lg border p-4 space-y-2">
                  <p className="font-medium text-sm">Uploaded Documents</p>
                  <div className="space-y-1 text-sm">
                    <div className="flex items-center gap-2">
                      {uploadedDocs.idFront ? (
                        <CheckCircle2 className="h-4 w-4 text-green-500" />
                      ) : (
                        <XCircle className="h-4 w-4 text-red-500" />
                      )}
                      <span>ID Front</span>
                    </div>
                    <div className="flex items-center gap-2">
                      {uploadedDocs.idBack ? (
                        <CheckCircle2 className="h-4 w-4 text-green-500" />
                      ) : (
                        <span className="h-4 w-4" />
                      )}
                      <span className="text-muted-foreground">
                        ID Back (optional)
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      {uploadedDocs.selfie ? (
                        <CheckCircle2 className="h-4 w-4 text-green-500" />
                      ) : (
                        <XCircle className="h-4 w-4 text-red-500" />
                      )}
                      <span>Selfie</span>
                    </div>
                  </div>
                </div>

                <div className="text-sm text-muted-foreground">
                  By submitting, you agree to our Terms of Service and Privacy
                  Policy.
                </div>
              </CardContent>
            </>
          )}

          <CardFooter className="flex justify-between">
            {step > 1 ? (
              <Button type="button" variant="outline" onClick={prevStep}>
                Back
              </Button>
            ) : (
              <div />
            )}

            {step < 4 ? (
              <Button type="button" onClick={nextStep}>
                Next
              </Button>
            ) : (
              <Button
                type="submit"
                disabled={
                  isSubmitting || !uploadedDocs.idFront || !uploadedDocs.selfie
                }
              >
                {isSubmitting && (
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                )}
                Submit Verification
              </Button>
            )}
          </CardFooter>
        </form>
      </Card>
    </div>
  );
}
