from django.conf import settings
from django.db import models

from .level import Level
from .create_update_model import CreateUpdateModel


class Subject(CreateUpdateModel):
    name = models.CharField(max_length=200)
    description = models.TextField()
    level = models.ForeignKey(Level, on_delete=models.PROTECT)
    teachers = models.ManyToManyField(
        settings.AUTH_USER_MODEL, related_name="subjects", blank=True
    )

    def __str__(self):
        return self.name
