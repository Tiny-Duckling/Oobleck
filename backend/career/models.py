from django.db import models
from django.utils.translation import gettext_lazy

from django.conf import settings


class CreateUpdateModel(models.Model):
    created_by = models.ForeignKey(settings.AUTH_USER_MODEL, on_delete=models.PROTECT)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_by = models.ForeignKey(
        settings.AUTH_USER_MODEL, on_delete=models.PROTECT, related_name="+"
    )
    updated_at = models.DateTimeField(auto_now=True)

    class Meta:
        abstract = True


class CreateModel(models.Model):
    created_by = models.ForeignKey(settings.AUTH_USER_MODEL, on_delete=models.PROTECT)
    created_at = models.DateTimeField(auto_now_add=True)

    class Meta:
        abstract = True


class Opening(CreateUpdateModel):
    title = models.CharField(max_length=100)
    vacancies = models.IntegerField()
    responsibilities = models.TextField()

    class JobType(models.TextChoices):
        full_time = "FT", gettext_lazy("Full Time")
        part_time = "PT", gettext_lazy("Part Time")
        contract = "CT", gettext_lazy("Contract")
        internship = "IS", gettext_lazy("Intern")

    job_type = models.CharField(
        max_length=20, choices=JobType.choices, default=JobType.full_time
    )

    time_commitment = models.CharField(max_length=100)
    remuneration = models.CharField(max_length=100)
    duration = models.CharField(max_length=100)
    location = models.CharField(max_length=100)
    required_qualifications = models.TextField()
    preferred_qualifications = models.TextField()
    deadline = models.DateField()

    def __str__(self):
        return self.title


class Application(CreateModel):
    opening = models.ForeignKey(Opening, on_delete=models.PROTECT)
    resume = models.FileField(null=True, blank=True)


class ApplicationVerdict(CreateUpdateModel):
    application = models.ForeignKey(Application, on_delete=models.PROTECT)
    Verdict = models.TextChoices("Verdict", "ACCEPTED REJECTED")
    verdict = models.CharField(
        max_length=10, choices=Verdict.choices, null=True, blank=True
    )
