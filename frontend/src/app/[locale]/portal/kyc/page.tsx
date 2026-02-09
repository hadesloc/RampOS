"use client";

import { useState, useEffect, useCallback } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { Clock, CheckCircle2, XCircle, AlertCircle, Loader2, Upload, Camera } from "lucide-react";
import { KYCStatus, kycApi } from "@/lib/portal-api";
import { useAuth } from "@/contexts/auth-context";
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
import { KYCProgress } from "@/components/portal/kyc-progress";
import { PageHeader } from "@/components/layout/page-header";
import { PageContainer } from "@/components/layout/page-container";
import { useRouter } from "@/navigation";
import { useTranslations } from "next-intl";

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
  const t = useTranslations('Portal.kyc');
  const tCommon = useTranslations('Common');
  const tPortal = useTranslations('Portal.dashboard');

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
    } catch {
      // Failed to fetch KYC status silently
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
    } catch {
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
  // if (authLoading || isLoadingStatus) {
  //   return (
  //     <div className="container max-w-2xl py-10">
  //       <div className="flex items-center justify-center py-20">
  //         <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
  //       </div>
  //     </div>
  //   );
  // }

  // Show KYC status if already submitted
  if (kycStatus && kycStatus.status !== "NONE") {
    return (
      <PageContainer>
        <PageHeader title={t('title')} description={t('description')} />
        <Card>
          <CardContent className="pt-6">
            <KYCProgress
                currentStep={4}
                steps={[
                    { label: t('step_1'), completed: true },
                    { label: t('step_2'), completed: true },
                    { label: t('step_3'), completed: true },
                    { label: t('step_4'), completed: true }
                ]}
                status={kycStatus.status}
            />
            {kycStatus.status === "PENDING" && (
              <div className="flex flex-col items-center text-center space-y-4 py-8">
                <div className="rounded-full bg-yellow-100 p-4 dark:bg-yellow-900/30">
                  <Clock className="h-12 w-12 text-yellow-600 dark:text-yellow-400" />
                </div>
                <div className="space-y-2">
                  <h2 className="text-xl font-semibold">{t('pending')}</h2>
                  <p className="text-muted-foreground max-w-md">
                    {tPortal('kyc_pending')}
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
                  <h2 className="text-xl font-semibold">{t('verified')}</h2>
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
                  <h2 className="text-xl font-semibold">{t('failed')}</h2>
                  <p className="text-muted-foreground max-w-md">
                    {tPortal('kyc_rejected')}
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
                  {tCommon('try_again')}
                </Button>
              </div>
            )}
          </CardContent>
        </Card>
      </PageContainer>
    );
  }

  const steps = [
    { label: t('step_1'), completed: step > 1 },
    { label: t('step_2'), completed: step > 2 },
    { label: t('step_3'), completed: step > 3 },
    { label: t('step_4'), completed: step > 4 }
  ];

  return (
    <PageContainer>
      <PageHeader title={t('title')} description={t('description')} />

      <div className="max-w-3xl mx-auto space-y-6">
      <KYCProgress
        currentStep={step}
        steps={steps}
        status={kycStatus?.status || 'NONE'}
      />

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
                <CardTitle>{t('step_1')}</CardTitle>
                <CardDescription>
                  {t('step_1_desc')}
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label htmlFor="firstName">{t('first_name')}</Label>
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
                    <Label htmlFor="lastName">{t('last_name')}</Label>
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
                  <Label htmlFor="dob">{t('dob')}</Label>
                  <Input id="dob" type="date" {...register("dob")} />
                  {errors.dob && (
                    <p className="text-sm text-red-500">
                      {errors.dob?.message as string}
                    </p>
                  )}
                </div>
                <div className="space-y-2">
                  <Label htmlFor="address">{t('address')}</Label>
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
                  <Label htmlFor="idDocumentType">{t('id_type')}</Label>
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
                <CardTitle>{t('step_2')}</CardTitle>
                <CardDescription>
                  Upload clear pictures of your {formData.idDocumentType?.toLowerCase().replace("_", " ") || "ID"}.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-6">
                <div className="space-y-4">
                  <div className="space-y-2">
                    <Label>{t('id_front')}</Label>
                    <div className="border-2 border-dashed rounded-lg p-6 text-center">
                      {uploadedDocs.idFront ? (
                        <div className="flex items-center justify-center gap-2 text-green-600">
                          <CheckCircle2 className="h-5 w-5" />
                          <span>{uploadedDocs.idFront.name}</span>
                        </div>
                      ) : uploadProgress.idFront ? (
                        <div className="flex items-center justify-center gap-2">
                          <Loader2 className="h-5 w-5 animate-spin" />
                          <span>{tCommon('loading')}</span>
                        </div>
                      ) : (
                        <label className="cursor-pointer">
                          <Upload className="h-8 w-8 mx-auto mb-2 text-muted-foreground" />
                          <span className="text-sm text-muted-foreground">
                            {t('upload_front')}
                          </span>
                          <input
                            id="idFrontUpload"
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
                    <Label>{t('id_back')}</Label>
                    <div className="border-2 border-dashed rounded-lg p-6 text-center">
                      {uploadedDocs.idBack ? (
                        <div className="flex items-center justify-center gap-2 text-green-600">
                          <CheckCircle2 className="h-5 w-5" />
                          <span>{uploadedDocs.idBack.name}</span>
                        </div>
                      ) : uploadProgress.idBack ? (
                        <div className="flex items-center justify-center gap-2">
                          <Loader2 className="h-5 w-5 animate-spin" />
                          <span>{tCommon('loading')}</span>
                        </div>
                      ) : (
                        <label className="cursor-pointer">
                          <Upload className="h-8 w-8 mx-auto mb-2 text-muted-foreground" />
                          <span className="text-sm text-muted-foreground">
                            {t('upload_back')}
                          </span>
                          <input
                            id="idBackUpload"
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
                <CardTitle>{t('step_3')}</CardTitle>
                <CardDescription>
                  {t('step_3_desc')}
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
                      <span>{tCommon('loading')}</span>
                    </div>
                  ) : (
                    <label className="cursor-pointer">
                      <Camera className="h-12 w-12 mx-auto mb-2 text-muted-foreground" />
                      <span className="text-sm text-muted-foreground block">
                        {t('upload_selfie')}
                      </span>
                      <span className="text-xs text-muted-foreground block mt-1">
                        Make sure your face is clearly visible
                      </span>
                      <input
                        id="selfieUpload"
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
                <CardTitle>{t('step_4')}</CardTitle>
                <CardDescription>
                  {t('step_4_desc')}
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="rounded-lg border p-4 space-y-2">
                  <div className="grid grid-cols-3 gap-2 text-sm">
                    <span className="font-medium text-muted-foreground">
                      {t('first_name')}:
                    </span>
                    <span className="col-span-2">
                      {formData.firstName} {formData.lastName}
                    </span>

                    <span className="font-medium text-muted-foreground">
                      {t('dob')}:
                    </span>
                    <span className="col-span-2">{formData.dob}</span>

                    <span className="font-medium text-muted-foreground">
                      {t('address')}:
                    </span>
                    <span className="col-span-2">{formData.address}</span>

                    <span className="font-medium text-muted-foreground">
                      {t('id_type')}:
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
                      <span>{t('id_front')}</span>
                    </div>
                    <div className="flex items-center gap-2">
                      {uploadedDocs.idBack ? (
                        <CheckCircle2 className="h-4 w-4 text-green-500" />
                      ) : (
                        <span className="h-4 w-4" />
                      )}
                      <span className="text-muted-foreground">
                        {t('id_back')} (optional)
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      {uploadedDocs.selfie ? (
                        <CheckCircle2 className="h-4 w-4 text-green-500" />
                      ) : (
                        <XCircle className="h-4 w-4 text-red-500" />
                      )}
                      <span>{t('step_3')}</span>
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
                {tCommon('back')}
              </Button>
            ) : (
              <div />
            )}

            {step < 4 ? (
              <Button type="button" onClick={nextStep}>
                {tCommon('next')}
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
                {tCommon('submit')}
              </Button>
            )}
          </CardFooter>
        </form>
      </Card>
      </div>
    </PageContainer>
  );
}
