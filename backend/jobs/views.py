from django.utils import timezone
from rest_framework import viewsets

from .mixins import AutoUserMixin
from .models import Opening, Application
from .permissions import IsStaffOrReadOnly
from .serializers import OpeningSerializer, ApplicationSerializer


class OpeningViewSet(AutoUserMixin, viewsets.ModelViewSet):
    queryset = Opening.objects.filter(deadline__gte=timezone.now())
    serializer_class = OpeningSerializer
    permission_classes = [IsStaffOrReadOnly]


class ApplicationViewSet(AutoUserMixin, viewsets.ModelViewSet):
    queryset = Application.objects.all()
    serializer_class = ApplicationSerializer
    permission_classes = [IsStaffOrReadOnly]

