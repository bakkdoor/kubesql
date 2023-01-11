SELECT
  kube-system,
  testing
FROM
  minikube
WHERE
  pod.status.phase = 'Running'
  OR deployment.metadata.name = 'hello-minikube'
  OR deployment.status.replicas = 1
