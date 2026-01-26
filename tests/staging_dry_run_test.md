
## Test Staging Deployment (Mock)
To verify the staging configuration, I'll run a dry-run test of the kustomization.

```bash
kubectl kustomize k8s/overlays/staging
```

I'll also run a dry-run test of the production configuration to ensure no regression.
```bash
kubectl kustomize k8s/overlays/prod
```
