from django.db import models
from django.utils.translation import gettext_lazy

from .question_group import QuestionGroup


class Question(models.Model):
    question_group = models.ForeignKey(QuestionGroup, on_delete=models.PROTECT)
    question_text = models.TextField()
    question_image = models.ImageField(null=True, blank=True)
    solution_description = models.TextField()
    marks = models.IntegerField()
    time_limit = models.DurationField()

    solution_text = models.TextField()
    solution_value = models.DecimalField(max_digits=30, decimal_places=8)

    class Type(models.TextChoices):
        MULTIPLE_CHOICE = "MC", gettext_lazy("Multiple Choice")
        TEXT_ANSWER = "TXT", gettext_lazy("Text Answer")
        NUMERICAL = "NUM", gettext_lazy("Numerical")

    type = models.CharField(
        max_length=3, choices=Type.choices, default=Type.MULTIPLE_CHOICE
    )

    def __str__(self):
        return self.question_text
