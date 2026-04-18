{{- define "cuba-ims.name" -}}
{{- default "cuba-ims" .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "cuba-ims.fullname" -}}
{{- printf "%s-%s" .Release.Name (include "cuba-ims.name" .) | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "cuba-ims.labels" -}}
app.kubernetes.io/name: {{ include "cuba-ims.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" }}
{{- end -}}

{{- define "cuba-ims.selectorLabels" -}}
app.kubernetes.io/name: {{ include "cuba-ims.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}
