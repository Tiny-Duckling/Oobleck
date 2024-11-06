from django.db import models

from .subject import Subject
from .create_update_model import CreateUpdateModel


class Category(CreateUpdateModel):
    name = models.CharField(max_length=200)
    description = models.TextField()
    subject = models.ForeignKey(Subject, on_delete=models.PROTECT)

    def __str__(self):
        return self.name
