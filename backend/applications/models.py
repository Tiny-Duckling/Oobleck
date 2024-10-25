from django.db import models
from django.utils.translation import gettext_lazy

from django.conf import settings


class ApplicationRequest(models.Model):
    job_title = models.CharField(max_length=100)
    vacancies = models.IntegerField()
    responsibilities = models.TextField()

    class JobType(models.TextChoices):
        full_time = 'FT', gettext_lazy('Full Time')
        part_time = 'PT', gettext_lazy('Part Time')
        contract = 'CT', gettext_lazy('Contract')
        internship = 'IS', gettext_lazy('Intern')

    job_type = models.CharField(max_length=20, choices=JobType.choices, default=JobType.full_time)

    time_commitment = models.CharField(max_length=100)
    remuneration = models.CharField(max_length=100)
    duration = models.CharField(max_length=100)
    location = models.CharField(max_length=100)
    required_qualifications = models.TextField()
    preferred_qualifications = models.TextField()
    deadline = models.DateField()

    created_by = models.ForeignKey(settings.AUTH_USER_MODEL, on_delete=models.PROTECT)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_by = models.ForeignKey(settings.AUTH_USER_MODEL, on_delete=models.PROTECT, related_name='+')
    updated_at = models.DateTimeField(auto_now=True)



