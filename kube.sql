SELECT
  kube-system,
  testing
FROM
  minikube
WHERE
  pod.status.phase = 'Running'
  OR deployment.metadata.name = 'hello-minikube'
  OR service.metadata.name = 'hello-minikube'
