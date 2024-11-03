from django.utils import timezone

from rest_framework import serializers
from .models import Opening, Application


class OpeningSerializer(serializers.ModelSerializer):
    class Meta:
        model = Opening
        fields = ('job_title', 'vacancies', 'responsibilities', 'job_type', 'time_commitment', 'remuneration',
                  'duration', 'location', 'required_qualifications', 'preferred_qualifications', 'deadline')


class ApplicationSerializer(serializers.ModelSerializer):
    def __init__(self, *args, **kwargs):
        super(ApplicationSerializer, self).__init__(*args, **kwargs)
        self.fields['opening'].queryset = Opening.objects.filter(deadline__gte=timezone.now())

    class Meta:
        model = Application
        fields = ('opening', 'resume')
