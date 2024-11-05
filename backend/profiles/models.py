from django.contrib.auth.models import AbstractUser
from django.db import models


class User(AbstractUser):
    credits = models.IntegerField(default=120)

    def __str__(self):
        return self.username


class WorkExperience(models.Model):
    user = models.ForeignKey(
        User, on_delete=models.PROTECT, related_name="work_experiences"
    )

    company = models.CharField(max_length=200)
    position = models.CharField(max_length=200)
    start_date = models.DateField()
    end_date = models.DateField(null=True, blank=True)
    description = models.TextField()

    def __str__(self):
        return self.company


class EducationalInstitution(models.Model):
    user = models.ForeignKey(
        User, on_delete=models.PROTECT, related_name="educational_institutions"
    )

    name = models.CharField(max_length=200)
    department = models.CharField(max_length=200)
    graduation_date = models.DateField()
    result = models.CharField(max_length=100)

    def __str__(self):
        return self.name


class Achievement(models.Model):
    user = models.ForeignKey(
        User, on_delete=models.PROTECT, related_name="achievements"
    )

    name = models.CharField(max_length=200)
    description = models.TextField()

    def __str__(self):
        return self.name
